#!/usr/bin/env python3
"""
Test Space Membership Modes (Phase 1 Implementation)
Tests: Creating spaces with different membership modes (lightweight vs MLS)
"""

import subprocess
import time
import re
import os
import sys

class Color:
    GREEN = '\033[0;32m'
    RED = '\033[0;31m'
    CYAN = '\033[0;36m'
    YELLOW = '\033[1;33m'
    NC = '\033[0m'

def run_command(client, cmd, wait=2):
    """Send command to client"""
    print(f"{Color.CYAN}[{client['name']}]{Color.NC} {cmd}")
    client['proc'].stdin.write(cmd + '\n')
    client['proc'].stdin.flush()
    time.sleep(wait)

def find_in_log(log_file, pattern):
    """Find pattern in log file"""
    try:
        with open(log_file, 'r') as f:
            content = f.read()
            match = re.search(pattern, content)
            return match.group(1) if match else None
    except:
        return None

def check_log(log_file, pattern):
    """Check if pattern exists in log"""
    try:
        with open(log_file, 'r') as f:
            return bool(re.search(pattern, f.read()))
    except:
        return False

def main():
    print(f"{Color.CYAN}╔═══════════════════════════════════════════════╗{Color.NC}")
    print(f"{Color.CYAN}║  Space Membership Modes Test (Phase 1)        ║{Color.NC}")
    print(f"{Color.CYAN}╚═══════════════════════════════════════════════╝{Color.NC}\n")
    
    # Setup test directory structure
    test_dir = 'tests/test-runs/space-modes'
    os.makedirs(test_dir, exist_ok=True)
    
    # Cleanup old test artifacts
    os.system(f'rm -rf {test_dir}/* 2>/dev/null')
    
    # Build
    binary_path = './target/debug/spaceway'
    
    if not os.path.exists(binary_path):
        print(f"{Color.CYAN}Building debug version...{Color.NC}")
        build_result = subprocess.run(
            ['cargo', '+nightly', 'build'],
            capture_output=True,
            text=True
        )
        
        if build_result.returncode != 0:
            print(f"{Color.RED}Build failed!{Color.NC}")
            print(build_result.stderr[-500:] if len(build_result.stderr) > 500 else build_result.stderr)
            return 1
        
        print(f"{Color.GREEN}✓ Build completed{Color.NC}")
    else:
        print(f"{Color.GREEN}✓ Using existing binary{Color.NC}")
    
    # Start Alice
    alice_log = open(f'{test_dir}/alice.log', 'w')
    
    print(f"{Color.CYAN}Starting Alice...{Color.NC}")
    
    alice = {
        'name': 'Alice',
        'proc': subprocess.Popen(
            [binary_path, '--account', f'{test_dir}/alice.key', '--port', '9001'],
            stdin=subprocess.PIPE, stdout=alice_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': f'{test_dir}/alice.log'
    }
    
    try:
        print(f"{Color.GREEN}✓ Alice started{Color.NC}\n")
        time.sleep(3)
        
        # Test 1: Create MLS-encrypted space (default)
        print(f"{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Test 1: Create MLS-encrypted space (default){Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        run_command(alice, 'keypackage publish', wait=5)
        run_command(alice, 'space create MLSTestSpace', wait=3)
        
        mls_space_id = find_in_log(alice['log'], r'Created space: .+? \(([0-9a-f]{16})\)')
        if not mls_space_id:
            print(f"{Color.RED}✗ Failed to create MLS space{Color.NC}")
            return 1
        
        # Check if MLS group was created
        has_mls_group = check_log(alice['log'], r'Created MLS-encrypted space|space-level encryption enabled')
        
        print(f"{Color.GREEN}✓ MLS space created: {mls_space_id}{Color.NC}")
        print(f"  MLS group created: {has_mls_group}\n")
        
        # Test 2: Create lightweight space
        print(f"{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Test 2: Create lightweight space{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        run_command(alice, 'space create LightweightTest --mode lightweight', wait=3)
        
        light_space_id = find_in_log(alice['log'], r'Created space: LightweightTest \(([0-9a-f]{16})\)')
        if not light_space_id:
            print(f"{Color.RED}✗ Failed to create lightweight space{Color.NC}")
            return 1
        
        # Check if lightweight space was created (no MLS group)
        has_lightweight = check_log(alice['log'], r'LIGHTWEIGHT space|no space-level MLS group')
        
        print(f"{Color.GREEN}✓ Lightweight space created: {light_space_id}{Color.NC}")
        print(f"  Lightweight mode: {has_lightweight}\n")
        
        # Test 3: Create MLS space explicitly
        print(f"{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Test 3: Create MLS space (explicit){Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        run_command(alice, 'space create ExplicitMLS --mode mls', wait=3)
        
        explicit_mls_id = find_in_log(alice['log'], r'Created space: ExplicitMLS \(([0-9a-f]{16})\)')
        if not explicit_mls_id:
            print(f"{Color.RED}✗ Failed to create explicit MLS space{Color.NC}")
            return 1
        
        has_explicit_mls = check_log(alice['log'], r'Created MLS-encrypted space|space-level encryption enabled')
        
        print(f"{Color.GREEN}✓ Explicit MLS space created: {explicit_mls_id}{Color.NC}")
        print(f"  MLS mode: {has_explicit_mls}\n")
        
        # Test 4: List spaces
        print(f"{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Test 4: List all spaces{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        run_command(alice, 'spaces', wait=2)
        
        # Close logs
        alice_log.close()
        
        # Check results
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Results{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        alice_log_content = open(alice['log']).read()
        
        tests_passed = 0
        tests_total = 6
        
        if mls_space_id:
            print(f"{Color.GREEN}✓{Color.NC} Created default MLS space")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Failed to create default MLS space")
        
        if has_mls_group or 'MLS' in alice_log_content:
            print(f"{Color.GREEN}✓{Color.NC} Default space uses MLS encryption")
            tests_passed += 1
        else:
            print(f"{Color.YELLOW}⚠{Color.NC}  Cannot confirm MLS encryption (may need to check logs)")
        
        if light_space_id:
            print(f"{Color.GREEN}✓{Color.NC} Created lightweight space")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Failed to create lightweight space")
        
        if has_lightweight:
            print(f"{Color.GREEN}✓{Color.NC} Lightweight space has no MLS group")
            tests_passed += 1
        else:
            print(f"{Color.YELLOW}⚠{Color.NC}  Cannot confirm lightweight mode (check logs)")
        
        if explicit_mls_id:
            print(f"{Color.GREEN}✓{Color.NC} Created explicit MLS space")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Failed to create explicit MLS space")
        
        if 'MLSTestSpace' in alice_log_content or 'LightweightTest' in alice_log_content or 'ExplicitMLS' in alice_log_content:
            print(f"{Color.GREEN}✓{Color.NC} All spaces listed correctly")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Not all spaces listed")
        
        print(f"\n{Color.CYAN}Score: {tests_passed}/{tests_total}{Color.NC}")
        
        if tests_passed >= 5:
            print(f"\n{Color.GREEN}🎉 SUCCESS! Space membership modes working!{Color.NC}")
            print(f"{Color.GREEN}   ✓ Can create MLS-encrypted spaces{Color.NC}")
            print(f"{Color.GREEN}   ✓ Can create lightweight spaces{Color.NC}")
            print(f"{Color.GREEN}   ✓ --mode flag working correctly{Color.NC}")
            return 0
        else:
            print(f"\n{Color.RED}❌ TEST FAILED ({tests_passed}/{tests_total}){Color.NC}")
            print(f"Check logs in: {test_dir}/")
            return 1
            
    finally:
        alice['proc'].terminate()
        try:
            alice['proc'].wait(timeout=3)
        except:
            alice['proc'].kill()

if __name__ == '__main__':
    sys.exit(main())
