"""
Utility for compiling regex patterns to SystemVerilog from pytest.

This module provides a simple interface for invoking the Rust Thompson NFA 
compiler from within pytest tests.
"""

import subprocess
import os
from pathlib import Path


def compile_regex_to_verilog(pattern: str, module_name: str = "test_regex") -> str:
    """
    Compile a regex pattern to SystemVerilog using the Rust binary.
    
    Args:
        pattern: Regular expression pattern
        module_name: Name for the generated Verilog module
        
    Returns:
        SystemVerilog code as string
        
    Raises:
        RuntimeError: If compilation fails
    """
    try:
        # Run the Rust compiler
        result = subprocess.run(
            ["cargo", "run", "--bin", "thompson_nfa_compiler", "--", pattern, module_name],
            cwd=".",
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if result.returncode != 0:
            raise RuntimeError(f"Compilation failed: {result.stderr}")
        
        # Read the generated SystemVerilog file
        output_file = f"{module_name}.sv"
        if not os.path.exists(output_file):
            raise RuntimeError(f"Expected output file {output_file} was not created")
        
        with open(output_file, 'r') as f:
            verilog_code = f.read()
        
        return verilog_code
    
    except subprocess.TimeoutExpired:
        raise RuntimeError("Compilation timed out")
    except FileNotFoundError:
        raise RuntimeError("Rust compiler not found. Make sure you're in the project directory.")


def compile_and_save(pattern: str, module_name: str, output_path: str = None) -> Path:
    """
    Compile regex pattern and save to specific path.
    
    Args:
        pattern: Regular expression pattern
        module_name: SystemVerilog module name
        output_path: Where to save the file (optional)
        
    Returns:
        Path to saved SystemVerilog file
    """
    verilog_code = compile_regex_to_verilog(pattern, module_name)
    
    if output_path is None:
        output_path = f"{module_name}.sv"
    
    output_file = Path(output_path)
    output_file.write_text(verilog_code)
    
    return output_file