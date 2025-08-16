"""
Cocotb test module for possessive quantifier patterns.

This module tests possessive quantifiers like L++, a*+, etc.
"""

import cocotb
from dfs_matcher import dfs_match_pattern


@cocotb.test()
async def possessive_quantifier_patterns(dut):
    """Test possessive quantifier L++ pattern matching."""
    
    # Test patterns for L++
    test_cases = [
        ("L", True),        # Single L should match L++
        ("LL", True),       # Multiple L should match L++
        ("LLL", True),      # Multiple L should match L++
        ("LLLL", True),     # Multiple L should match L++
        ("", False),        # Empty should not match L++
        ("a", False),       # Non-L should not match
        ("aL", False),      # L not at start should not match
        ("La", False),      # L+ followed by non-L should not match (possessive)
        ("LLa", False),     # LL+ followed by non-L should not match (possessive)
    ]
    
    results = {}
    failures = []
    
    for pattern, expected in test_cases:
        try:
            matched = await dfs_match_pattern(dut, pattern)
            results[pattern] = matched
            
            if matched == expected:
                dut._log.info(f"✓ Pattern '{pattern}': expected {expected}, got {matched}")
            else:
                dut._log.error(f"✗ Pattern '{pattern}': expected {expected}, got {matched}")
                failures.append(pattern)
                
        except Exception as e:
            dut._log.error(f"✗ Pattern '{pattern}' threw exception: {e}")
            failures.append(pattern)
    
    # Report results
    passed = len(test_cases) - len(failures) 
    dut._log.info(f"Possessive quantifier test results: {passed}/{len(test_cases)} passed")
    
    if failures:
        raise AssertionError(f"Failed patterns: {failures}")
    
    dut._log.info("All possessive quantifier tests passed!")