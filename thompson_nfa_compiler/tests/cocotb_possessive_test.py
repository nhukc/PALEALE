"""
Cocotb test module for character class patterns.

This module contains the actual @cocotb.test() functions that will be
executed by the cocotb runner from pytest.
"""

import cocotb
from dfs_matcher import dfs_match_pattern


@cocotb.test()
async def character_class_patterns(dut):
    """Test character class [abc] pattern matching."""

    # Test patterns for [abc]
    test_cases = [
        ("a", True),
        ("aa", True),
        ("aaa", True),
        ("ab", False),
        ("aaaaab", False),
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
    dut._log.info(f"Character class test results: {passed}/{len(test_cases)} passed")

    if failures:
        raise AssertionError(f"Failed patterns: {failures}")

    dut._log.info("All character class tests passed!")
