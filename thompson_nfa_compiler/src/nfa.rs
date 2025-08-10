use std::collections::HashSet;

/// A state ID in the NFA
pub type StateId = usize;

/// A two-character transition that matches based on current and next character
#[derive(Debug, Clone, PartialEq)]
pub struct TwoCharTransition {
    /// The current character to match (None means any character)
    pub current: Option<char>,
    /// The next character for lookahead (None means any character or end-of-input)
    pub lookahead: Option<char>,
    /// The target state after consuming the current character
    pub target: StateId,
}

impl TwoCharTransition {
    /// Create a simple single-character transition
    pub fn char(ch: char, target: StateId) -> Self {
        TwoCharTransition {
            current: Some(ch),
            lookahead: None,
            target,
        }
    }

    /// Create a transition with lookahead
    pub fn char_with_lookahead(current: char, lookahead: char, target: StateId) -> Self {
        TwoCharTransition {
            current: Some(current),
            lookahead: Some(lookahead),
            target,
        }
    }

    /// Create a single-character transition (possessive behavior comes from structure, not flags)
    pub fn char_possessive(ch: char, target: StateId) -> Self {
        TwoCharTransition {
            current: Some(ch),
            lookahead: None,
            target,
        }
    }

    /// Create a dot (any character) transition
    pub fn dot(target: StateId) -> Self {
        TwoCharTransition {
            current: None,
            lookahead: None,
            target,
        }
    }

    /// Create a dot transition with lookahead
    pub fn dot_with_lookahead(lookahead: char, target: StateId) -> Self {
        TwoCharTransition {
            current: None,
            lookahead: Some(lookahead),
            target,
        }
    }
}

/// A Thompson NFA state with two-character transitions
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// A state with two-character transitions
    Transitions {
        /// List of possible transitions from this state
        transitions: Vec<TwoCharTransition>,
    },
    
    /// Epsilon transition (no input consumed)
    Epsilon { 
        next: StateId 
    },
    
    /// Split state with multiple epsilon transitions
    Split { 
        targets: Vec<StateId> 
    },
    
    /// Match state (accepting)
    Match,
    
    /// Rejected state (never matches, dead end)
    Rejected,
}

/// Fragment of an NFA with start and end states
#[derive(Debug, Clone)]
pub struct Fragment {
    pub start: StateId,
    pub end: StateId,
}

/// A Thompson NFA with two-character transitions
#[derive(Debug, Clone, PartialEq)]
pub struct NFA {
    /// All states in the NFA
    pub states: Vec<State>,
    /// Starting state
    pub start: StateId,
    /// Set of accepting states
    pub accepting: HashSet<StateId>,
    /// Next available state ID
    next_id: StateId,
}

impl NFA {
    /// Create a new empty NFA
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            start: 0,
            accepting: HashSet::new(),
            next_id: 0,
        }
    }
    
    /// Add a new state and return its ID
    pub fn add_state(&mut self, state: State) -> StateId {
        let id = self.next_id;
        self.states.push(state);
        self.next_id += 1;
        id
    }
    
    /// Create an epsilon transition state
    pub fn epsilon(&mut self, next: StateId) -> StateId {
        self.add_state(State::Epsilon { next })
    }
    
    /// Create a split state with multiple targets
    pub fn split(&mut self, targets: Vec<StateId>) -> StateId {
        self.add_state(State::Split { targets })
    }
    
    /// Create a transition state with a single two-character transition
    pub fn transition_state(&mut self, transition: TwoCharTransition) -> StateId {
        self.add_state(State::Transitions {
            transitions: vec![transition],
        })
    }
    
    /// Create a transition state with multiple two-character transitions
    pub fn transitions_state(&mut self, transitions: Vec<TwoCharTransition>) -> StateId {
        self.add_state(State::Transitions { transitions })
    }
    
    /// Create a match state
    pub fn match_state(&mut self) -> StateId {
        let id = self.add_state(State::Match);
        self.accepting.insert(id);
        id
    }
    
    /// Create a rejected state
    pub fn rejected_state(&mut self) -> StateId {
        self.add_state(State::Rejected)
    }
    
    /// Connect two states with an epsilon transition
    pub fn connect(&mut self, from: StateId, to: StateId) {
        if from >= self.states.len() {
            return;
        }
        
        match &mut self.states[from] {
            State::Epsilon { next } => *next = to,
            State::Split { targets } => targets.push(to),
            State::Transitions { transitions } => {
                // Update all transitions that have a target of usize::MAX (unpatched) to point to 'to'
                for transition in transitions {
                    if transition.target == usize::MAX {
                        transition.target = to;
                    }
                }
            },
            State::Match => {}, // Match states don't have outgoing transitions
            State::Rejected => {}, // Rejected states don't have outgoing transitions
        }
    }
    
    /// Get epsilon closure of a set of states
    pub fn epsilon_closure(&self, states: &HashSet<StateId>) -> HashSet<StateId> {
        let mut closure = states.clone();
        let mut stack: Vec<StateId> = states.iter().cloned().collect();
        
        while let Some(state_id) = stack.pop() {
            if state_id >= self.states.len() {
                continue;
            }
            
            match &self.states[state_id] {
                State::Epsilon { next } => {
                    if closure.insert(*next) {
                        stack.push(*next);
                    }
                },
                State::Split { targets } => {
                    for &target in targets {
                        if closure.insert(target) {
                            stack.push(target);
                        }
                    }
                },
                _ => {}, // Non-epsilon states (transitions, match, rejected) don't contribute to epsilon closure
            }
        }
        
        closure
    }
    
    /// Check if any state in the set is accepting
    pub fn is_accepting(&self, states: &HashSet<StateId>) -> bool {
        states.iter().any(|&state| self.accepting.contains(&state))
    }

    /// Get all possible two-character transitions from a set of states
    pub fn get_two_char_transitions(&self, states: &HashSet<StateId>) -> Vec<TwoCharTransition> {
        let mut transitions = Vec::new();
        
        for &state_id in states {
            if state_id >= self.states.len() {
                continue;
            }
            
            if let State::Transitions { transitions: state_transitions } = &self.states[state_id] {
                transitions.extend(state_transitions.clone());
            }
        }
        
        transitions
    }
}

impl Default for NFA {
    fn default() -> Self {
        Self::new()
    }
}