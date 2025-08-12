"""
Cocotb test module for literal string patterns.

This module tests literal string matching like "abc".
"""

import cocotb
from dfs_matcher import dfs_match_pattern


@cocotb.test()
async def literal_string_patterns(dut):
    """Test literal string 'abc' pattern matching."""
    
    # Test patterns for literal "abc"
    test_cases = [
        ("abc", True),
        ("ab", False),
        ("abcd", False),  # Should not match - exact match only
        ("xyz", False),
        ("", False),
        ("a", False),
        ("bc", False),
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
    dut._log.info(f"Literal string test results: {passed}/{len(test_cases)} passed")
    
    if failures:
        raise AssertionError(f"Failed patterns: {failures}")
    
    dut._log.info("All literal string tests passed!")