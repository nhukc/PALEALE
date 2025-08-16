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
    dut._log.info(f"=== DFS matching pattern '{pattern}' ===")
    
    # Wait for circuit to settle, then read start state
    await Timer(1, units='ns')
    start_state = int(dut.start_state.value)
    dut._log.info(f"Circuit start state: {start_state}")
    
    stack = [(start_state, 0)]  # (state, position) - start at circuit's start state
    cycle = 0
    
    while stack and cycle < max_cycles:
        current_state, pos = stack.pop()
        cycle += 1
        
        dut._log.info(f"Cycle {cycle}: state {current_state}, pos {pos}")
        
        # Set up inputs for current position
        if pos < len(pattern):
            char = pattern[pos]
            next_char = pattern[pos + 1] if pos + 1 < len(pattern) else None
            
            dut.first_char.value = char_to_utf32(char)
            if next_char is not None:
                dut.second_char.value = char_to_utf32(next_char)
                dut.second_valid.value = 1
                dut._log.info(f"  Input: '{char}' (next: '{next_char}')")
            else:
                dut.second_char.value = 0
                dut.second_valid.value = 0
                dut._log.info(f"  Input: '{char}' (no next)")
        else:
            # End of input
            dut.first_char.value = 0
            dut.second_char.value = 0
            dut.second_valid.value = 0
            dut._log.info(f"  Input: end of input")
        
        # Set current state and get circuit output
        dut.current_state.value = current_state
        await Timer(1, units='ns')
        
        next_state = int(dut.next_state.value)
        second_state = int(dut.second_state.value) if int(dut.enabled.value) else None
        consumed = int(dut.consumed.value)
        enabled = int(dut.enabled.value)
        
        dut._log.info(f"  Output: next_state={next_state}, second_state={second_state}, consumed={consumed}, enabled={enabled}")
        
        if consumed and pos < len(pattern):
            # Circuit consumed input, advance position
            new_pos = pos + 1
            dut._log.info(f"  Consumed input, new_pos={new_pos}")
            
            # Check if we're done and in match state
            if new_pos == len(pattern) and next_state == 0:
                dut._log.info(f"  MATCH FOUND! Reached end of input at MATCH state")
                return True
            
            # Add next states to stack
            if next_state != 1:  # Not rejected
                stack.append((next_state, new_pos))
                dut._log.info(f"  Added to stack: ({next_state}, {new_pos})")
            if second_state is not None and second_state != 1:
                stack.append((second_state, new_pos))
                dut._log.info(f"  Added to stack: ({second_state}, {new_pos})")
                
        elif not consumed:
            # Epsilon transition, don't advance position
            dut._log.info(f"  Epsilon transition, pos stays {pos}")
            if pos == len(pattern) and next_state == 0:
                dut._log.info(f"  MATCH FOUND! Epsilon to MATCH state at end of input")
                return True
                
            # Add next states to stack
            if next_state != 1:  # Not rejected
                stack.append((next_state, pos))
                dut._log.info(f"  Added to stack: ({next_state}, {pos})")
            if second_state is not None and second_state != 1:
                stack.append((second_state, pos))
                dut._log.info(f"  Added to stack: ({second_state}, {pos})")
        else:
            dut._log.info(f"  Circuit wanted to consume but at end of input - path failed")
    
    if cycle >= max_cycles:
        dut._log.error(f"DFS hit max cycles ({max_cycles})")
    else:
        dut._log.info(f"DFS exhausted all paths after {cycle} cycles")
    
    return False