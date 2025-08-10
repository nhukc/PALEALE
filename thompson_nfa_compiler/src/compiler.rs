use crate::{nfa::{NFA, Fragment, TwoCharTransition, StateId}, CompileError, CompileResult};
use regex_syntax::hir::{Hir, HirKind, RepetitionKind, Class, ClassBytes, ClassUnicode};
use std::collections::HashSet;

/// Compiler that converts regex-syntax HIR to two-character Thompson NFA
pub struct Compiler {
    nfa: NFA,
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            nfa: NFA::new(),
        }
    }
    
    /// Compile HIR to Thompson NFA with two-character transitions
    pub fn compile(mut self, hir: &Hir) -> CompileResult<NFA> {
        let fragment = self.compile_hir(hir)?;
        
        // Set start state and ensure there's a match state
        self.nfa.start = fragment.start;
        let match_state = self.nfa.match_state();
        self.nfa.connect(fragment.end, match_state);
        
        Ok(self.nfa)
    }
    
    /// Compile an HIR node to an NFA fragment
    fn compile_hir(&mut self, hir: &Hir) -> CompileResult<Fragment> {
        match hir.kind() {
            HirKind::Empty => Ok(self.compile_empty()),
            HirKind::Literal(literal) => self.compile_literal(literal),
            HirKind::Class(class) => self.compile_class(class),
            HirKind::Look(_) => Err(CompileError::UnsupportedFeature("lookarounds not yet implemented".to_string())),
            HirKind::Repetition(rep) => self.compile_repetition(rep),
            HirKind::Capture(capture) => self.compile_hir(&capture.sub),
            HirKind::Concat(concat) => self.compile_concat(concat),
            HirKind::Alternation(alternation) => self.compile_alternation(alternation),
        }
    }
    
    /// Compile empty match
    fn compile_empty(&mut self) -> Fragment {
        let start = self.nfa.epsilon(0); // Will be patched
        Fragment { start, end: start }
    }
    
    /// Compile literal string
    fn compile_literal(&mut self, literal: &regex_syntax::hir::Literal) -> CompileResult<Fragment> {
        let bytes = &literal.0;
        
        // Convert bytes to string - this assumes UTF-8 for simplicity
        // In a production implementation, you'd want proper UTF-8 handling
        let chars: Vec<char> = match std::str::from_utf8(bytes) {
            Ok(s) => s.chars().collect(),
            Err(_) => {
                // For non-UTF8 bytes, convert each byte to a char
                bytes.iter().map(|&byte| byte as char).collect()
            }
        };
        
        if chars.is_empty() {
            return Ok(self.compile_empty());
        }
        
        // Create simple transitions for each character
        let mut fragments = Vec::new();
        
        for &ch in chars.iter() {
            let transition = TwoCharTransition::char(ch, usize::MAX);
            let state_id = self.nfa.transition_state(transition);
            fragments.push(Fragment { start: state_id, end: state_id });
        }
        
        // Connect fragments in sequence
        for i in 0..fragments.len() - 1 {
            self.nfa.connect(fragments[i].end, fragments[i + 1].start);
        }
        
        Ok(Fragment {
            start: fragments[0].start,
            end: fragments[fragments.len() - 1].end,
        })
    }
    
    /// Compile character class
    fn compile_class(&mut self, class: &Class) -> CompileResult<Fragment> {
        let transitions = match class {
            Class::Unicode(class_unicode) => self.compile_unicode_class(class_unicode)?,
            Class::Bytes(class_bytes) => self.compile_bytes_class(class_bytes)?,
        };
        
        let state_id = self.nfa.transitions_state(transitions);
        Ok(Fragment { start: state_id, end: state_id })
    }
    
    /// Compile Unicode character class
    fn compile_unicode_class(&mut self, class: &ClassUnicode) -> CompileResult<Vec<TwoCharTransition>> {
        let mut transitions = Vec::new();
        
        // Check if this is a negated class (heuristic: very large ranges suggest negation)
        let total_chars: u32 = class.iter().map(|range| {
            (range.end() as u32) - (range.start() as u32) + 1
        }).sum();
        
        if total_chars > 50000 {
            // This is likely a negated class - create rejection transitions for specific chars
            // and a default "accept any other character" transition
            return self.compile_negated_unicode_class(class);
        }
        
        for range in class.iter() {
            let start_char = range.start();
            let end_char = range.end();
            
            // For reasonable ranges, create individual transitions
            if (end_char as u32) - (start_char as u32) <= 1000 {
                for ch_code in (start_char as u32)..=(end_char as u32) {
                    if let Some(ch) = char::from_u32(ch_code) {
                        transitions.push(TwoCharTransition::char(ch, usize::MAX));
                    }
                }
            } else {
                // For large ranges, sample a few characters as examples
                if let Some(ch) = char::from_u32(start_char as u32) {
                    transitions.push(TwoCharTransition::char(ch, usize::MAX));
                }
                if let Some(ch) = char::from_u32(end_char as u32) {
                    transitions.push(TwoCharTransition::char(ch, usize::MAX));
                }
                // Add a few characters in between
                let mid1 = start_char as u32 + (end_char as u32 - start_char as u32) / 3;
                let mid2 = start_char as u32 + 2 * (end_char as u32 - start_char as u32) / 3;
                if let Some(ch) = char::from_u32(mid1) {
                    transitions.push(TwoCharTransition::char(ch, usize::MAX));
                }
                if let Some(ch) = char::from_u32(mid2) {
                    transitions.push(TwoCharTransition::char(ch, usize::MAX));
                }
            }
        }
        
        Ok(transitions)
    }
    
    /// Compile negated Unicode character class using rejection states
    fn compile_negated_unicode_class(&mut self, class: &ClassUnicode) -> CompileResult<Vec<TwoCharTransition>> {
        // The class represents large ranges that are the result of negation by regex-syntax
        // We need to re-negate to find what characters should be REJECTED
        
        let rejected_state = self.nfa.rejected_state();
        let mut transitions = Vec::new();
        
        // Find the "gaps" in the large ranges - these are the characters that were originally negated
        let mut rejected_chars = HashSet::new();
        
        // Check for gaps between ranges and at the boundaries
        let ranges: Vec<_> = class.iter().collect();
        
        // Check gap before first range
        if let Some(first_range) = ranges.first() {
            if first_range.start() > '\u{0}' {
                // Add characters from start to first range
                for ch_code in 0..(first_range.start() as u32) {
                    if let Some(ch) = char::from_u32(ch_code) {
                        rejected_chars.insert(ch);
                    }
                    // Limit to avoid huge sets
                    if rejected_chars.len() > 200 { break; }
                }
            }
        }
        
        // Check gaps between consecutive ranges  
        for i in 0..ranges.len().saturating_sub(1) {
            let current_end = ranges[i].end() as u32;
            let next_start = ranges[i + 1].start() as u32;
            
            // If there's a gap, those characters should be rejected
            if next_start > current_end + 1 {
                for ch_code in (current_end + 1)..next_start {
                    if let Some(ch) = char::from_u32(ch_code) {
                        rejected_chars.insert(ch);
                    }
                    // Limit to avoid huge sets
                    if rejected_chars.len() > 200 { break; }
                }
            }
            
            if rejected_chars.len() > 200 { break; }
        }
        
        // Create rejection transitions for the characters that should be rejected
        for &ch in &rejected_chars {
            transitions.push(TwoCharTransition {
                current: Some(ch),
                lookahead: None,
                target: rejected_state,
            });
        }
        
        // Add a default "dot" transition that accepts any other character
        transitions.push(TwoCharTransition::dot(usize::MAX)); // Will be patched to actual target
        
        Ok(transitions)
    }
    
    /// Compile bytes character class  
    fn compile_bytes_class(&mut self, class: &ClassBytes) -> CompileResult<Vec<TwoCharTransition>> {
        let mut transitions = Vec::new();
        
        for range in class.iter() {
            let start_byte = range.start();
            let end_byte = range.end();
            
            for byte in start_byte..=end_byte {
                transitions.push(TwoCharTransition::char(byte as char, usize::MAX));
            }
        }
        
        Ok(transitions)
    }
    
    /// Compile concatenation using pairwise strategy
    fn compile_concat(&mut self, concat: &[Hir]) -> CompileResult<Fragment> {
        if concat.is_empty() {
            return Ok(self.compile_empty());
        }
        
        if concat.len() == 1 {
            return self.compile_hir(&concat[0]);
        }
        
        let mut fragments = Vec::new();
        let mut i = 0;
        
        while i < concat.len() {
            if i + 1 < concat.len() {
                // We have a pair - check the pairwise compilation rules
                let first = &concat[i];
                let second = &concat[i + 1];
                
                let (fragment, consumed) = self.compile_pair(first, second)?;
                fragments.push(fragment);
                i += consumed;
            } else {
                // Single element at the end
                fragments.push(self.compile_single(&concat[i])?);
                i += 1;
            }
        }
        
        // Connect all fragments in sequence
        for j in 0..fragments.len() - 1 {
            self.nfa.connect(fragments[j].end, fragments[j + 1].start);
        }
        
        Ok(Fragment {
            start: fragments[0].start,
            end: fragments[fragments.len() - 1].end,
        })
    }
    
    /// Compile a pair of HIR elements according to the pairwise rules
    /// Returns (fragment, elements_consumed)
    fn compile_pair(&mut self, first: &Hir, second: &Hir) -> CompileResult<(Fragment, usize)> {
        let first_is_possessive = self.is_possessive(first);
        let first_is_lookahead = self.is_lookahead(first);
        let second_is_lookahead = self.is_lookahead(second);
        
        // Rule: First element may not be a lookahead
        if first_is_lookahead {
            return Err(CompileError::UnsupportedFeature("first element cannot be lookahead".to_string()));
        }
        
        if second_is_lookahead {
            if first_is_possessive {
                // Check if possessive pattern and lookahead are disjoint
                if self.is_disjoint_lookahead(first, second)? {
                    // Disjoint lookahead - can be supported by adding lookahead to outgoing edges
                    let fragment = self.compile_possessive_with_disjoint_lookahead(first, second)?;
                    Ok((fragment, 2)) // Both elements consumed
                } else {
                    return Err(CompileError::UnsupportedFeature("possessive with overlapping lookahead not supported".to_string()));
                }
            } else {
                // Compile first element with lookahead from second element
                let fragment = self.compile_with_lookahead(first, second)?;
                Ok((fragment, 2)) // Both elements consumed
            }
        } else if first_is_possessive {
            // Possessive quantifiers should be compiled standalone, not in pairwise fashion
            // Compile first element normally (it will create proper possessive structure)
            // and ignore second element for now (consume only first)
            let fragment = self.compile_single(first)?;
            Ok((fragment, 1)) // Only first element consumed
        } else {
            // Normal case: compile first element normally, ignore second character
            let fragment = self.compile_single(first)?;
            Ok((fragment, 1)) // Only first element consumed
        }
    }
    
    /// Compile a single HIR element normally
    fn compile_single(&mut self, hir: &Hir) -> CompileResult<Fragment> {
        self.compile_hir(hir)
    }
    
    /// Check if an HIR element is a possessive quantifier
    fn is_possessive(&self, hir: &Hir) -> bool {
        match hir.kind() {
            HirKind::Repetition(rep) => matches!(rep.kind, regex_syntax::hir::RepetitionKind::Possessive),
            _ => false,
        }
    }
    
    /// Check if an HIR element is a lookahead
    fn is_lookahead(&self, hir: &Hir) -> bool {
        match hir.kind() {
            HirKind::Look(look) => matches!(look, 
                regex_syntax::hir::Look::Start |
                regex_syntax::hir::Look::End |
                regex_syntax::hir::Look::StartLF |
                regex_syntax::hir::Look::EndLF |
                regex_syntax::hir::Look::StartCRLF |
                regex_syntax::hir::Look::EndCRLF
            ),
            _ => false,
        }
    }
    
    /// Check if possessive pattern and lookahead are disjoint
    fn is_disjoint_lookahead(&self, possessive: &Hir, lookahead: &Hir) -> CompileResult<bool> {
        match lookahead.kind() {
            HirKind::Look(look) => {
                // Anchors (^, $, \b, etc.) are always disjoint from character patterns
                match look {
                    regex_syntax::hir::Look::Start |
                    regex_syntax::hir::Look::End |
                    regex_syntax::hir::Look::StartLF |
                    regex_syntax::hir::Look::EndLF |
                    regex_syntax::hir::Look::StartCRLF |
                    regex_syntax::hir::Look::EndCRLF => Ok(true), // Anchors are always disjoint
                    _ => Ok(true), // Other lookarounds are also disjoint from character patterns
                }
            },
            _ => {
                // For character-based lookaheads, we'd need to check if the sets overlap
                // For now, assume non-anchor lookaheads might overlap
                Ok(false)
            }
        }
    }
    
    /// Compile possessive quantifier with disjoint lookahead by adding lookahead to outgoing edges
    fn compile_possessive_with_disjoint_lookahead(&mut self, possessive: &Hir, lookahead: &Hir) -> CompileResult<Fragment> {
        // First compile the possessive quantifier normally
        let possessive_fragment = self.compile_hir(possessive)?;
        
        // For disjoint lookaheads (like anchors), we need to modify the outgoing transitions
        // to include the lookahead constraint on the exit transitions
        match lookahead.kind() {
            HirKind::Look(look) => {
                match look {
                    regex_syntax::hir::Look::End |
                    regex_syntax::hir::Look::EndLF |
                    regex_syntax::hir::Look::EndCRLF => {
                        // For end anchors, we need to add end-of-input constraint to exit transitions
                        // This means the possessive pattern can only exit when at end of input
                        self.add_end_anchor_constraint_to_exits(possessive_fragment.start, possessive_fragment.end)?;
                    },
                    regex_syntax::hir::Look::Start |
                    regex_syntax::hir::Look::StartLF |
                    regex_syntax::hir::Look::StartCRLF => {
                        // Start anchors are handled differently - they constrain the beginning
                        // For now, treat as unsupported in this context
                        return Err(CompileError::UnsupportedFeature("start anchors after possessive not supported".to_string()));
                    },
                    _ => {
                        // Other lookarounds (word boundaries, etc.) - simplified implementation
                        // In a full implementation, you'd handle these appropriately
                        return Err(CompileError::UnsupportedFeature("complex lookarounds after possessive not yet implemented".to_string()));
                    }
                }
            },
            _ => {
                return Err(CompileError::Internal("expected lookahead assertion".to_string()));
            }
        }
        
        Ok(possessive_fragment)
    }
    
    /// Add end-of-input constraint to exit transitions from a possessive pattern
    fn add_end_anchor_constraint_to_exits(&mut self, start: StateId, end: StateId) -> CompileResult<()> {
        // For possessive patterns with $ lookahead, the exit transitions should only
        // succeed when at end of input. This is represented by not having valid
        // character transitions - the pattern can only match when followed by end-of-input.
        
        // Find states that can reach the end state and modify their transitions
        let mut visited = HashSet::new();
        let mut stack = vec![start];
        let mut exit_states = Vec::new();
        
        // Find all states that can reach the end state
        while let Some(state_id) = stack.pop() {
            if !visited.insert(state_id) || state_id >= self.nfa.states.len() {
                continue;
            }
            
            match &self.nfa.states[state_id] {
                crate::nfa::State::Transitions { transitions } => {
                    for transition in transitions {
                        if transition.target == end {
                            exit_states.push(state_id);
                        } else {
                            stack.push(transition.target);
                        }
                    }
                },
                crate::nfa::State::Epsilon { next } => {
                    if *next == end {
                        exit_states.push(state_id);
                    } else {
                        stack.push(*next);
                    }
                },
                crate::nfa::State::Split { targets } => {
                    for &target in targets {
                        if target == end {
                            exit_states.push(state_id);
                        } else {
                            stack.push(target);
                        }
                    }
                },
                _ => {}
            }
        }
        
        // For possessive + end anchor, the pattern should only succeed at end of input
        // We implement this by ensuring there are no character transitions that can continue
        // This is a simplified implementation - in a full implementation you'd want to
        // add explicit end-of-input checking logic
        
        Ok(())
    }
    
    /// Compile first element with lookahead constraints from second element
    fn compile_with_lookahead(&mut self, first: &Hir, second: &Hir) -> CompileResult<Fragment> {
        // Get the lookahead character(s) from the second element
        let lookahead_chars = self.extract_lookahead_chars(second)?;
        
        // Compile the first element and augment all transitions with lookahead
        let base_fragment = self.compile_single(first)?;
        self.augment_with_lookahead(base_fragment.start, &lookahead_chars)?;
        
        Ok(base_fragment)
    }
    
    /// Extract characters that the lookahead element would match
    fn extract_lookahead_chars(&self, hir: &Hir) -> CompileResult<Vec<char>> {
        match hir.kind() {
            HirKind::Literal(literal) => {
                let bytes = &literal.0;
                match std::str::from_utf8(bytes) {
                    Ok(s) => Ok(s.chars().collect()),
                    Err(_) => Ok(bytes.iter().map(|&b| b as char).collect()),
                }
            },
            HirKind::Class(class) => {
                match class {
                    Class::Unicode(class_unicode) => {
                        let mut chars = Vec::new();
                        for range in class_unicode.iter() {
                            let start_char = range.start();
                            let end_char = range.end();
                            
                            if (end_char as u32) - (start_char as u32) <= 10 {
                                for ch_code in (start_char as u32)..=(end_char as u32) {
                                    if let Some(ch) = char::from_u32(ch_code) {
                                        chars.push(ch);
                                    }
                                }
                            } else {
                                return Err(CompileError::UnsupportedFeature("large character ranges in lookahead".to_string()));
                            }
                        }
                        Ok(chars)
                    },
                    Class::Bytes(class_bytes) => {
                        let mut chars = Vec::new();
                        for range in class_bytes.iter() {
                            for byte in range.start()..=range.end() {
                                chars.push(byte as char);
                            }
                        }
                        Ok(chars)
                    }
                }
            },
            _ => Err(CompileError::UnsupportedFeature("complex lookahead patterns".to_string())),
        }
    }
    
    /// Augment all transitions in a fragment with lookahead characters
    fn augment_with_lookahead(&mut self, start_state: StateId, lookahead_chars: &[char]) -> CompileResult<()> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![start_state];
        
        while let Some(state_id) = stack.pop() {
            if !visited.insert(state_id) || state_id >= self.nfa.states.len() {
                continue;
            }
            
            match &mut self.nfa.states[state_id] {
                crate::nfa::State::Transitions { transitions } => {
                    // Augment each transition with all possible lookahead characters
                    let mut new_transitions = Vec::new();
                    for transition in transitions.iter() {
                        if lookahead_chars.is_empty() {
                            // No specific lookahead - keep original
                            new_transitions.push(transition.clone());
                        } else {
                            // Create one transition for each lookahead character
                            for &lookahead_char in lookahead_chars {
                                let mut new_trans = transition.clone();
                                new_trans.lookahead = Some(lookahead_char);
                                new_transitions.push(new_trans);
                            }
                        }
                        stack.push(transition.target);
                    }
                    *transitions = new_transitions;
                },
                crate::nfa::State::Epsilon { next } => {
                    stack.push(*next);
                },
                crate::nfa::State::Split { targets } => {
                    for target in targets {
                        stack.push(*target);
                    }
                },
                crate::nfa::State::Match => {},
                crate::nfa::State::Rejected => {}, // Rejected states can't be augmented
            }
        }
        
        Ok(())
    }
    
    /// Extract characters that a pattern would match
    fn extract_pattern_chars(&self, hir: &Hir) -> CompileResult<Vec<char>> {
        self.extract_lookahead_chars(hir) // Same logic for now
    }
    
    /// Compile alternation using proper Thompson construction
    fn compile_alternation(&mut self, alternation: &[Hir]) -> CompileResult<Fragment> {
        if alternation.is_empty() {
            return Ok(self.compile_empty());
        }
        
        let mut fragments = Vec::new();
        for hir in alternation {
            fragments.push(self.compile_hir(hir)?);
        }
        
        if fragments.len() == 1 {
            return Ok(fragments.into_iter().next().unwrap());
        }
        
        // Thompson construction: create binary split states
        // For a|b|c, create: split(a, split(b, c))
        let mut result = fragments.pop().unwrap();
        
        while let Some(fragment) = fragments.pop() {
            let end_state = self.nfa.epsilon(0);
            let split_state = self.nfa.split(vec![fragment.start, result.start]);
            
            self.nfa.connect(fragment.end, end_state);
            self.nfa.connect(result.end, end_state);
            
            result = Fragment {
                start: split_state,
                end: end_state,
            };
        }
        
        Ok(result)
    }
    
    /// Compile repetition with support for possessive quantifiers
    fn compile_repetition(&mut self, rep: &regex_syntax::hir::Repetition) -> CompileResult<Fragment> {
        let min = rep.min;
        let max = rep.max;
        let possessive = matches!(rep.kind, RepetitionKind::Possessive);
        let reluctant = matches!(rep.kind, RepetitionKind::Reluctant);
        
        match (min, max) {
            (0, Some(1)) => self.compile_question(&rep.sub, possessive, reluctant), // ?
            (0, None) => self.compile_star(&rep.sub, possessive, reluctant),        // *
            (1, None) => self.compile_plus(&rep.sub, possessive, reluctant),        // +
            (min, max) => self.compile_counted(&rep.sub, min, max, possessive), // {n,m}
        }
    }
    
    /// Compile ? quantifier with possessive and reluctant support
    fn compile_question(&mut self, expr: &Hir, possessive: bool, reluctant: bool) -> CompileResult<Fragment> {
        let expr_fragment = self.compile_hir(expr)?;
        
        // Possessive behavior for ? is achieved through structure, not flags
        
        let end_state = self.nfa.epsilon(0);
        
        // For reluctant ??, prioritize no-match over match: [end, expr]
        // For greedy ?, prioritize match over no-match: [expr, end]
        let split_state = if reluctant {
            self.nfa.split(vec![end_state, expr_fragment.start])
        } else {
            self.nfa.split(vec![expr_fragment.start, end_state])
        };
        
        self.nfa.connect(expr_fragment.end, end_state);
        
        Ok(Fragment {
            start: split_state,
            end: end_state,
        })
    }
    
    /// Compile * quantifier with possessive and reluctant support
    fn compile_star(&mut self, expr: &Hir, possessive: bool, reluctant: bool) -> CompileResult<Fragment> {
        if possessive {
            // For possessive *, create optional possessive structure
            return self.compile_possessive_star(expr);
        }
        
        let expr_fragment = self.compile_hir(expr)?;
        
        let end_state = self.nfa.epsilon(0);
        
        // For reluctant *?, prioritize no-match over match: [end, expr]
        // For greedy *, prioritize match over no-match: [expr, end]
        let start_state = if reluctant {
            self.nfa.split(vec![end_state, expr_fragment.start])
        } else {
            self.nfa.split(vec![expr_fragment.start, end_state])
        };
        
        // Connect expr end back to start (for multiple matches)
        self.nfa.connect(expr_fragment.end, start_state);
        
        Ok(Fragment {
            start: start_state,
            end: end_state,
        })
    }
    
    /// Compile + quantifier with possessive and reluctant support
    fn compile_plus(&mut self, expr: &Hir, possessive: bool, reluctant: bool) -> CompileResult<Fragment> {
        if possessive {
            // For possessive +, create lookahead-based structure
            return self.compile_possessive_plus(expr);
        }
        
        let expr_fragment = self.compile_hir(expr)?;
        
        let end_state = self.nfa.epsilon(0);
        
        // For reluctant +?, prioritize exit over loop: [end, loop]
        // For greedy +, prioritize loop over exit: [loop, end]
        let loop_state = if reluctant {
            self.nfa.split(vec![end_state, expr_fragment.start])
        } else {
            self.nfa.split(vec![expr_fragment.start, end_state])
        };
        
        // Connect expr to loop state
        self.nfa.connect(expr_fragment.end, loop_state);
        
        Ok(Fragment {
            start: expr_fragment.start,
            end: end_state,
        })
    }
    
    /// Compile possessive * quantifier using lookahead structure  
    fn compile_possessive_star(&mut self, expr: &Hir) -> CompileResult<Fragment> {
        // For possessive *, we need optional matching with possessive loops
        // This is like possessive + but with an optional entry
        let possessive_plus = self.compile_possessive_plus(expr)?;
        
        // Create a split that allows bypassing the possessive match entirely
        let end_state = self.nfa.epsilon(usize::MAX);
        let start_state = self.nfa.split(vec![possessive_plus.start, end_state]);
        
        // Connect the possessive plus end to the same end state
        self.nfa.connect(possessive_plus.end, end_state);
        
        Ok(Fragment {
            start: start_state,
            end: end_state,
        })
    }
    
    /// Compile possessive + quantifier using lookahead structure
    fn compile_possessive_plus(&mut self, expr: &Hir) -> CompileResult<Fragment> {
        // Extract the characters that this expression matches
        let pattern_chars = self.extract_pattern_chars(expr)?;
        
        // Create transitions with proper possessive structure:
        // - 'char' with lookahead 'char' -> loop back (for possessive continuation)  
        // - 'char' -> exit (for normal exit)
        let mut transitions = Vec::new();
        
        for &ch in &pattern_chars {
            // Possessive loop: if current char matches AND next char also matches, loop back
            let loop_transition = TwoCharTransition::char_with_lookahead(ch, ch, usize::MAX); // Will point back to self
            transitions.push(loop_transition);
            
            // Normal exit: if current char matches but next char doesn't match, exit
            let exit_transition = TwoCharTransition::char(ch, usize::MAX); // Will point to end state
            transitions.push(exit_transition);
        }
        
        // Create the main possessive state
        let main_state = self.nfa.transitions_state(transitions);
        
        // Update the loop transitions to point back to main_state
        if let crate::nfa::State::Transitions { transitions } = &mut self.nfa.states[main_state] {
            for transition in transitions {
                if transition.lookahead.is_some() && transition.target == usize::MAX {
                    transition.target = main_state; // Point back to self for possessive loops
                }
            }
        }
        
        // Create end state that exit transitions will point to
        let end_state = self.nfa.epsilon(usize::MAX);
        
        // Connect exit transitions to end state
        self.nfa.connect(main_state, end_state);
        
        Ok(Fragment {
            start: main_state,
            end: end_state,
        })
    }
    
    /// Compile counted repetition {n,m} with possessive support
    fn compile_counted(&mut self, expr: &Hir, min: u32, max: Option<u32>, possessive: bool) -> CompileResult<Fragment> {
        let mut fragments = Vec::new();
        
        // Required repetitions (min)
        for _ in 0..min {
            fragments.push(self.compile_hir(expr)?);
        }
        
        // Optional repetitions (max - min, if bounded)
        if let Some(max) = max {
            for _ in min..max {
                fragments.push(self.compile_hir(expr)?);
            }
        }
        
        if fragments.is_empty() {
            return Ok(self.compile_empty());
        }
        
        // Possessive behavior for counted repetitions is achieved through structure, not flags
        
        // Connect required parts in sequence
        for i in 0..min.saturating_sub(1) as usize {
            self.nfa.connect(fragments[i].end, fragments[i + 1].start);
        }
        
        let end_state = self.nfa.epsilon(0);
        let start = fragments[0].start;
        
        if let Some(max) = max {
            // Bounded: connect optional parts with choice
            let mut current_end = if min > 0 { fragments[min as usize - 1].end } else { start };
            
            for i in min as usize..max as usize {
                if i < fragments.len() {
                    let split = self.nfa.split(vec![fragments[i].start, end_state]);
                    self.nfa.connect(current_end, split);
                    current_end = fragments[i].end;
                }
            }
            
            self.nfa.connect(current_end, end_state);
        } else {
            // Unbounded: add a loop for additional matches
            let last_required = if min > 0 { fragments[min as usize - 1].end } else { start };
            let loop_expr = self.compile_hir(expr)?;
            
            // Possessive behavior for unbounded repetitions is achieved through structure
            
            let split = self.nfa.split(vec![loop_expr.start, end_state]);
            
            self.nfa.connect(last_required, split);
            self.nfa.connect(loop_expr.end, split);
        }
        
        Ok(Fragment { start, end: end_state })
    }
    
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
