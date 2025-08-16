"""
Cocotb integration tests using the Python runner API.

This follows the cocotb 1.8+ pattern for integrating with pytest.
"""

import os
import shutil
import pytest
from pathlib import Path
from cocotb.runner import get_runner

from tests.compile_regex import compile_and_save


class TestCocotbRunner:
    """Test regex patterns using cocotb's Python runner."""

    @pytest.fixture(autouse=True)
    def setup(self, tmp_path):
        """Set up test environment."""
        self.test_dir = tmp_path
        self.sim = os.getenv("SIM", "icarus")

    def run_cocotb_test(self, tmp_path, pattern: str, module_name: str, cocotb_test_file: str):
        """
        Shared runner for cocotb tests.

        Args:
            tmp_path: Pytest temporary directory
            pattern: Regex pattern to test
            module_name: SystemVerilog module name
            cocotb_test_file: Name of cocotb test file (without .py)
        """
        # Compile regex to SystemVerilog
        sv_file = compile_and_save(
            pattern,
            module_name,
            str(tmp_path / f"{module_name}.sv")
        )

        # Copy cocotb test module to tmp_path so cocotb can find it
        test_module_src = Path(__file__).parent / f"{cocotb_test_file}.py"
        test_module_dst = tmp_path / f"test_{module_name}.py"  # cocotb expects test_ prefix
        shutil.copy2(test_module_src, test_module_dst)

        # Copy shared DFS matcher
        dfs_src = Path(__file__).parent / "dfs_matcher.py"
        dfs_dst = tmp_path / "dfs_matcher.py"
        shutil.copy2(dfs_src, dfs_dst)

        # Set up cocotb runner
        runner = get_runner(self.sim)

        # Build the design
        runner.build(
            sources=[sv_file],
            hdl_toplevel=module_name,
            build_dir=tmp_path / "build",
            always=True,
        )

        # Run the cocotb test
        runner.test(
            hdl_toplevel=module_name,
            test_module=f"test_{module_name}",
            test_dir=str(tmp_path),
        )

    def test_simple_character_class_runner(self, tmp_path):
        """Test [abc] pattern with cocotb runner."""
        self.run_cocotb_test(tmp_path, "[abc]", "char_test", "cocotb_char_test")

    def test_literal_string_runner(self, tmp_path):
        """Test literal string pattern with cocotb runner."""
        self.run_cocotb_test(tmp_path, "abc", "literal_test", "cocotb_literal_test")

    def test_alternation_runner(self, tmp_path):
        """Test alternation pattern with cocotb runner."""
        self.run_cocotb_test(tmp_path, "a|b|c", "alternation_test", "cocotb_char_test")

    def test_possessive_quantifier_runner(self, tmp_path):
        """Test possessive quantifier."""
        self.run_cocotb_test(tmp_path, "L++", "possessive_test", "cocotb_possessive_test")

if __name__ == "__main__":
    # Direct usage without pytest
    test_direct_runner()
