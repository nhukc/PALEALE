use crate::nfa::{NFA, TwoCharTransition, StateId};
use std::collections::HashSet;

/// A matcher that executes a two-character Thompson NFA against input
pub struct Matcher<'a> {
    nfa: &'a NFA,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched: bool,
    pub start: usize,
    pub end: usize,
}

impl<'a> Matcher<'a> {
    /// Create a new matcher for the given NFA
    pub fn new(nfa: &'a NFA) -> Self {
        Self { nfa }
    }
    
    /// Find the first match in the input string
    pub fn find(&self, input: &str) -> Option<MatchResult> {
        let chars: Vec<char> = input.chars().collect();
        
        // Try matching at each position
        for start in 0..=chars.len() {
            if let Some(end) = self.match_at(&chars, start) {
                return Some(MatchResult {
                    matched: true,
                    start,
                    end,
                });
            }
        }
        
        None
    }
    
    /// Check if the entire input matches
    pub fn is_match(&self, input: &str) -> bool {
        let chars: Vec<char> = input.chars().collect();
        self.match_at(&chars, 0) == Some(chars.len())
    }
    
    /// Try to match at a specific position
    fn match_at(&self, chars: &[char], start: usize) -> Option<usize> {
        let mut current_states = HashSet::new();
        current_states.insert(self.nfa.start);
        
        // Get epsilon closure of starting states
        current_states = self.nfa.epsilon_closure(&current_states);
        
        let mut position = start;
        
        // Check if we're already in an accepting state (handles empty matches)
        if self.nfa.is_accepting(&current_states) && start <= chars.len() {
            return Some(position);
        }
        
        // Process each character
        while position < chars.len() && !current_states.is_empty() {
            let current_char = chars[position];
            let next_char = if position + 1 < chars.len() {
                Some(chars[position + 1])
            } else {
                None
            };
            
            let next_states = self.step_states(&current_states, current_char, next_char);
            
            if next_states.is_empty() {
                break;
            }
            
            current_states = self.nfa.epsilon_closure(&next_states);
            position += 1;
            
            // Check if we're in an accepting state after consuming this character
            if self.nfa.is_accepting(&current_states) {
                return Some(position);
            }
            
            // Possessive behavior is handled structurally through lookahead, not flags
        }
        
        // Final check for accepting state
        if self.nfa.is_accepting(&current_states) {
            Some(position)
        } else {
            None
        }
    }
    
    /// Step from current states using a character with lookahead
    fn step_states(&self, current_states: &HashSet<StateId>, current_char: char, next_char: Option<char>) -> HashSet<StateId> {
        let mut next_states = HashSet::new();
        
        // Get all possible transitions from current states
        let transitions = self.nfa.get_two_char_transitions(current_states);
        
        for transition in transitions {
            if self.transition_matches(&transition, current_char, next_char) {
                next_states.insert(transition.target);
            }
        }
        
        next_states
    }
    
    /// Check if a transition matches the current character and lookahead
    fn transition_matches(&self, transition: &TwoCharTransition, current_char: char, next_char: Option<char>) -> bool {
        // Check current character predicate
        if !transition.current.matches(current_char) {
            return false;
        }
        
        // Check lookahead predicate
        match (&transition.lookahead, next_char) {
            (None, _) => true, // No lookahead constraint
            (Some(lookahead_pred), Some(actual)) => lookahead_pred.matches(actual),
            (Some(_), None) => false, // Expected lookahead but at end of input
        }
    }
    
    
    /// Find all matches in the input (greedy)
    pub fn find_all(&self, input: &str) -> Vec<MatchResult> {
        let mut matches = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut start = 0;
        
        while start < chars.len() {
            let _remaining = &chars[start..];
            
            // Try to find a match starting at this position
            if let Some(match_len) = self.match_at(&chars, start) {
                matches.push(MatchResult {
                    matched: true,
                    start,
                    end: match_len,
                });
                
                // Move past this match
                start = match_len.max(start + 1);
            } else {
                start += 1;
            }
        }
        
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nfa::TwoCharTransition;
    
    #[test]
    fn test_simple_match() {
        let mut nfa = NFA::new();
        
        // Create a simple NFA that matches "ab"
        let a_state = nfa.transition_state(TwoCharTransition::char_with_lookahead('a', 'b', 0));
        let b_state = nfa.transition_state(TwoCharTransition::char('b', 0));
        let match_state = nfa.match_state();
        
        nfa.connect(a_state, b_state);
        nfa.connect(b_state, match_state);
        nfa.start = a_state;
        
        let matcher = Matcher::new(&nfa);
        
        assert!(matcher.is_match("ab"));
        assert!(!matcher.is_match("ac"));
        assert!(!matcher.is_match("a"));
    }
    
    #[test]
    fn test_simple_char_match() {
        let mut nfa = NFA::new();
        
        // Create NFA that matches "a"
        let a_state = nfa.transition_state(TwoCharTransition::char('a', 0));
        let match_state = nfa.match_state();
        
        nfa.connect(a_state, match_state);
        nfa.start = a_state;
        
        let matcher = Matcher::new(&nfa);
        
        assert!(matcher.is_match("a"));
        assert!(!matcher.is_match("b"));
    }
    
    #[test]
    fn test_dot_match() {
        let mut nfa = NFA::new();
        
        // Create NFA that matches any single character
        let dot_state = nfa.transition_state(TwoCharTransition::dot(0));
        let match_state = nfa.match_state();
        
        nfa.connect(dot_state, match_state);
        nfa.start = dot_state;
        
        let matcher = Matcher::new(&nfa);
        
        assert!(matcher.is_match("a"));
        assert!(matcher.is_match("x"));
        assert!(!matcher.is_match(""));
        assert!(!matcher.is_match("ab"));
    }
}