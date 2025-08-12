"""
Shared DFS pattern matching implementation for Thompson NFA circuits.
"""

import cocotb
from cocotb.triggers import Timer


def char_to_utf32(c: str) -> int:
    """Convert character to 32-bit UTF-32 codepoint."""
    return ord(c)


async def dfs_match_pattern(dut, pattern: str, max_cycles: int = 50) -> bool:
    """
    Simple DFS-based pattern matching for Thompson NFA circuits.
    
    Args:
        dut: Device under test (cocotb DUT)
        pattern: String pattern to match
        max_cycles: Maximum cycles to prevent infinite loops
        
    Returns:
        True if pattern matches, False otherwise
    """
    stack = [(2, 0)]  # (state, position) - start at state 2, position 0
    cycle = 0
    
    while stack and cycle < max_cycles:
        current_state, pos = stack.pop()
        cycle += 1
        
        # Set up inputs for current position
        if pos < len(pattern):
            char = pattern[pos]
            next_char = pattern[pos + 1] if pos + 1 < len(pattern) else None
            
            dut.first_char.value = char_to_utf32(char)
            if next_char is not None:
                dut.second_char.value = char_to_utf32(next_char)
                dut.second_valid.value = 1
            else:
                dut.second_char.value = 0
                dut.second_valid.value = 0
        else:
            # End of input
            dut.first_char.value = 0
            dut.second_char.value = 0
            dut.second_valid.value = 0
        
        # Set current state and get circuit output
        dut.current_state.value = current_state
        await Timer(1, units='ns')
        
        next_state = int(dut.next_state.value)
        second_state = int(dut.second_state.value) if int(dut.enabled.value) else None
        consumed = int(dut.consumed.value)
        
        if consumed and pos < len(pattern):
            # Circuit consumed input, advance position
            new_pos = pos + 1
            
            # Check if we're done and in match state
            if new_pos == len(pattern) and next_state == 0:
                return True
            
            # Add next states to stack
            if next_state != 1:  # Not rejected
                stack.append((next_state, new_pos))
            if second_state is not None and second_state != 1:
                stack.append((second_state, new_pos))
                
        elif not consumed:
            # Epsilon transition, don't advance position
            if pos == len(pattern) and next_state == 0:
                return True
                
            # Add next states to stack
            if next_state != 1:  # Not rejected
                stack.append((next_state, pos))
            if second_state is not None and second_state != 1:
                stack.append((second_state, pos))
    
    return False