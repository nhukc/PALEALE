use thompson_nfa_compiler::Compiler;
use regex_syntax::ParserBuilder;

fn main() {
    println!("Two-Character Thompson NFA Compiler - NFA Structure Demo");
    println!("=========================================================");
    
    // Test cases demonstrating the two-character transition capability
    let test_patterns = vec![
        "ab",
        "a*", 
        "a+",
        "a?",
        "a|b",
        "[abc]",
        "[sdmt]",  // Simple character class - should NOT be possessive
        "a++ab",
        "a*+ab",
        "a+?ab",
        "[ab]++ab",
        "[^abc]",
        "[^ab]++",  // Test possessive with negated class
        // Progressive tests to isolate the possessive bleed issue
        "[sdmt]|ll",           // Simple alternation - no possessive
        "[sdmt]|L++",          // Mix normal + possessive
        "(?:[sdmt])",          // Non-capturing group
        "(?:[sdmt]|ll)",       // Non-capturing group with alternation
        "(?:[sdmt]|ll)|L++",   // Non-capturing + possessive in alternation
        " ?L++",               // Optional space + possessive L
        " ?L++| ?N++",         // Two possessive branches
        "[sdmt]| ?L++",        // Normal + possessive in alternation
        "(?:[sdmt]|ll|ve|re)", // The first part of complex pattern
        "(?:[sdmt]|ll|ve|re)| ?L++", // First part + one possessive branch
        "(?:[sdmt]|ll|ve|re)| ?\\p{L}++| ?\\p{N}++| ?[^\\s\\p{L}\\p{N}]++|\\s++$|\\s+\\S|\\s"
    ];
    
    for pattern in test_patterns {
        println!("\n=== Pattern: '{}' ===", pattern);
        
        // Parse the regex pattern
        let hir = match ParserBuilder::new().build().parse(pattern) {
            Ok(hir) => hir,
            Err(e) => {
                println!("Failed to parse pattern: {}", e);
                continue;
            }
        };
        
        // Debug: Print the HIR structure
        println!("HIR: {:?}", hir);
        
        // Compile to two-character Thompson NFA
        let nfa = match Compiler::new().compile(&hir) {
            Ok(nfa) => nfa,
            Err(e) => {
                println!("Failed to compile: {}", e);
                continue;
            }
        };
        
        print_nfa(&nfa);
    }
    
    // Demonstrate manual NFA construction with two-character transitions
    demonstrate_manual_construction();
}

fn print_nfa(nfa: &thompson_nfa_compiler::NFA) {
    println!("Start state: {}", nfa.start);
    println!("Accepting states: {:?}", nfa.accepting);
    println!("States:");
    
    for (id, state) in nfa.states.iter().enumerate() {
        print!("  {}: ", id);
        match state {
            thompson_nfa_compiler::nfa::State::Match => {
                println!("MATCH");
            },
            thompson_nfa_compiler::nfa::State::Rejected => {
                println!("REJECTED");
            },
            thompson_nfa_compiler::nfa::State::Epsilon { next } => {
                println!("ε -> {}", next);
            },
            thompson_nfa_compiler::nfa::State::Split { targets } => {
                println!("SPLIT -> {:?}", targets);
            },
            thompson_nfa_compiler::nfa::State::Transitions { transitions } => {
                println!("TRANSITIONS:");
                for (i, trans) in transitions.iter().enumerate() {
                    print!("    {}: ", i);
                    
                    // Print current predicate
                    match &trans.current {
                        thompson_nfa_compiler::nfa::CharacterPredicate::Any => print!("."),
                        thompson_nfa_compiler::nfa::CharacterPredicate::Char(c) => print!("'{}'", c),
                        thompson_nfa_compiler::nfa::CharacterPredicate::CharSet(set) => {
                            print!("[");
                            let chars: Vec<_> = set.iter().collect();
                            for (i, ch) in chars.iter().enumerate() {
                                if i > 0 { print!(""); }
                                print!("{}", ch);
                            }
                            print!("]");
                        },
                        thompson_nfa_compiler::nfa::CharacterPredicate::NotCharSet(set) => {
                            print!("[^");
                            let chars: Vec<_> = set.iter().collect();
                            for (i, ch) in chars.iter().enumerate() {
                                if i > 0 { print!(""); }
                                print!("{}", ch);
                            }
                            print!("]");
                        },
                    }
                    
                    // Print lookahead predicate
                    if let Some(lookahead) = &trans.lookahead {
                        print!(" with lookahead ");
                        match lookahead {
                            thompson_nfa_compiler::nfa::CharacterPredicate::Any => print!("."),
                            thompson_nfa_compiler::nfa::CharacterPredicate::Char(c) => print!("'{}'", c),
                            thompson_nfa_compiler::nfa::CharacterPredicate::CharSet(set) => {
                                print!("[");
                                let chars: Vec<_> = set.iter().collect();
                                for (i, ch) in chars.iter().enumerate() {
                                    if i > 0 { print!(""); }
                                    print!("{}", ch);
                                }
                                print!("]");
                            },
                            thompson_nfa_compiler::nfa::CharacterPredicate::NotCharSet(set) => {
                                print!("[^");
                                let chars: Vec<_> = set.iter().collect();
                                for (i, ch) in chars.iter().enumerate() {
                                    if i > 0 { print!(""); }
                                    print!("{}", ch);
                                }
                                print!("]");
                            },
                        }
                    }
                    
                    println!(" -> {}", trans.target);
                }
            }
        }
    }
}

fn demonstrate_manual_construction() {
    use thompson_nfa_compiler::nfa::{NFA, TwoCharTransition};
    
    println!("\n=== Manual Construction Examples ===");
    
    // Example 1: Simple possessive 'a*'
    println!("\n--- Possessive 'a*' (manual construction) ---");
    let mut possessive_nfa = NFA::new();
    
    let match_state = possessive_nfa.match_state();
    let a_transition = TwoCharTransition::char_possessive('a', 0); // Possessive 'a'
    let a_state = possessive_nfa.transition_state(a_transition);
    let split_state = possessive_nfa.split(vec![a_state, match_state]);
    
    // Connect a_state back to split for repetition
    possessive_nfa.connect(a_state, split_state);
    possessive_nfa.start = split_state;
    
    print_nfa(&possessive_nfa);
    
    // Example 2: Two-character transition with explicit lookahead
    println!("\n--- 'a' with lookahead for 'b', then 'b' (manual construction) ---");
    let mut lookahead_nfa = NFA::new();
    
    let ab_transition = TwoCharTransition::char_with_lookahead('a', 'b', 0);
    let b_transition = TwoCharTransition::char('b', 0);
    let ab_state = lookahead_nfa.transition_state(ab_transition);
    let b_state = lookahead_nfa.transition_state(b_transition);
    let match_state = lookahead_nfa.match_state();
    
    lookahead_nfa.connect(ab_state, b_state);
    lookahead_nfa.connect(b_state, match_state);
    lookahead_nfa.start = ab_state;
    
    print_nfa(&lookahead_nfa);
}
