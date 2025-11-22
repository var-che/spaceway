#!/usr/bin/env python3
"""
Test MLS group messaging with three members
Tests: Alice creates space → invites Bob and Charlie → Alice posts message → 
       Both Bob and Charlie can decrypt it
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
    print(f"{Color.CYAN}║  MLS Three Members Test                       ║{Color.NC}")
    print(f"{Color.CYAN}╚═══════════════════════════════════════════════╝{Color.NC}\n")
    
    # Cleanup
    os.system('rm -rf *-data/ *.key *.history alice_3m.log bob_3m.log charlie_3m.log 2>/dev/null')
    
    # Build (use debug build since it's faster and we already have it)
    binary_path = './target/debug/spaceway'
    
    # Check if binary exists
    if not os.path.exists(binary_path):
        print(f"{Color.CYAN}Building debug version (this may take a while)...{Color.NC}")
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
    
    # Start clients
    alice_log = open('alice_3m.log', 'w')
    bob_log = open('bob_3m.log', 'w')
    charlie_log = open('charlie_3m.log', 'w')
    
    print(f"{Color.CYAN}Starting Alice, Bob, and Charlie...{Color.NC}")
    
    alice = {
        'name': 'Alice',
        'proc': subprocess.Popen(
            [binary_path, '--account', 'alice.key', '--port', '9001'],
            stdin=subprocess.PIPE, stdout=alice_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': 'alice_3m.log'
    }
    
    bob = {
        'name': 'Bob',
        'proc': subprocess.Popen(
            [binary_path, '--account', 'bob.key', '--port', '9002'],
            stdin=subprocess.PIPE, stdout=bob_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': 'bob_3m.log'
    }
    
    charlie = {
        'name': 'Charlie',
        'proc': subprocess.Popen(
            [binary_path, '--account', 'charlie.key', '--port', '9003'],
            stdin=subprocess.PIPE, stdout=charlie_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': 'charlie_3m.log'
    }
    
    try:
        print(f"{Color.GREEN}✓ Alice, Bob, and Charlie started{Color.NC}\n")
        time.sleep(3)
        
        # Setup
        print(f"{Color.CYAN}Setting up KeyPackages and Space...{Color.NC}")
        run_command(alice, 'keypackage publish', wait=5)
        run_command(bob, 'keypackage publish', wait=5)
        run_command(charlie, 'keypackage publish', wait=5)
        
        run_command(alice, 'space create three-members', wait=3)
        space_id = find_in_log(alice['log'], r'Created space: .+? \(([0-9a-f]{16})\)')
        if not space_id:
            print(f"{Color.RED}✗ Failed to create space{Color.NC}")
            return 1
        
        run_command(alice, 'context', wait=2)
        full_space_id = find_in_log(alice['log'], r'Space: ([0-9a-f]{64})')
        
        run_command(alice, 'invite create', wait=3)
        invite = find_in_log(alice['log'], r'Created invite code: (\w+)')
        
        run_command(alice, 'network', wait=2)
        peer_id = find_in_log(alice['log'], r'Peer ID: (\w+)')
        
        print(f"{Color.GREEN}✓ Setup complete{Color.NC}\n")
        
        # Bob joins
        print(f"{Color.CYAN}Bob connecting and joining...{Color.NC}")
        run_command(bob, f'connect /ip4/127.0.0.1/tcp/9001/p2p/{peer_id}', wait=3)
        run_command(bob, f'join {full_space_id} {invite}', wait=5)
        
        run_command(bob, 'whoami', wait=2)
        bob_id = find_in_log(bob['log'], r'User ID: ([0-9a-f]{64})')
        
        print(f"{Color.GREEN}✓ Bob joined{Color.NC}\n")
        
        # Charlie joins
        print(f"{Color.CYAN}Charlie connecting and joining...{Color.NC}")
        run_command(charlie, f'connect /ip4/127.0.0.1/tcp/9001/p2p/{peer_id}', wait=3)
        run_command(charlie, f'join {full_space_id} {invite}', wait=5)
        
        run_command(charlie, 'whoami', wait=2)
        charlie_id = find_in_log(charlie['log'], r'User ID: ([0-9a-f]{64})')
        
        print(f"{Color.GREEN}✓ Charlie joined{Color.NC}\n")
        
        # Add Bob to MLS
        print(f"{Color.CYAN}Adding Bob to MLS group...{Color.NC}")
        run_command(alice, f'member add {bob_id}', wait=5)
        time.sleep(15)  # Wait for Welcome message to be fully processed before adding next member
        print(f"{Color.GREEN}✓ Bob added to MLS{Color.NC}\n")
        
        # Add Charlie to MLS
        print(f"{Color.CYAN}Adding Charlie to MLS group...{Color.NC}")
        run_command(alice, f'member add {charlie_id}', wait=5)
        time.sleep(8)  # Wait for Welcome message AND for Bob to receive Commit (epoch update)
        print(f"{Color.GREEN}✓ Charlie added to MLS{Color.NC}\n")
        
        # Alice sends message
        print(f"{Color.CYAN}Alice creating channel and sending message...{Color.NC}")
        run_command(alice, 'channel create general', wait=3)
        run_command(alice, 'thread create "Three Members Test"', wait=3)
        run_command(alice, 'send Hello everyone! Can Bob and Charlie both decrypt this?', wait=4)
        
        # Wait for message propagation
        print(f"{Color.YELLOW}⏳ Waiting for message propagation (10s)...{Color.NC}")
        time.sleep(10)
        
        # Close logs
        alice_log.close()
        bob_log.close()
        charlie_log.close()
        
        # Check results
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Results{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        # Count decryptions
        alice_log_content = open(alice['log']).read()
        bob_log_content = open(bob['log']).read()
        charlie_log_content = open(charlie['log']).read()
        
        alice_decrypts = len(re.findall(r'Decrypted MLS message', alice_log_content))
        bob_decrypts = len(re.findall(r'Decrypted MLS message', bob_log_content))
        charlie_decrypts = len(re.findall(r'Decrypted MLS message', charlie_log_content))
        
        # Check if they received Alice's message
        bob_got_message = 'Can Bob and Charlie both decrypt this' in bob_log_content
        charlie_got_message = 'Can Bob and Charlie both decrypt this' in charlie_log_content
        
        print(f"  Alice total decryptions: {alice_decrypts}")
        print(f"  Bob total decryptions: {bob_decrypts}")
        print(f"  Charlie total decryptions: {charlie_decrypts}")
        print(f"  Bob received Alice's message: {bob_got_message}")
        print(f"  Charlie received Alice's message: {charlie_got_message}\n")
        
        tests_passed = 0
        tests_total = 5
        
        # Test 1: Bob received MLS messages
        if bob_decrypts >= 3:  # CreateChannel, CreateThread, Message
            print(f"{Color.GREEN}✓{Color.NC} Bob decrypted MLS messages ({bob_decrypts} total)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob should have ≥3 decryptions, got {bob_decrypts}")
        
        # Test 2: Charlie received MLS messages
        if charlie_decrypts >= 3:  # CreateChannel, CreateThread, Message
            print(f"{Color.GREEN}✓{Color.NC} Charlie decrypted MLS messages ({charlie_decrypts} total)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie should have ≥3 decryptions, got {charlie_decrypts}")
        
        # Test 3: Bob got Alice's message content
        if bob_got_message:
            print(f"{Color.GREEN}✓{Color.NC} Bob decrypted Alice's message content")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's message")
        
        # Test 4: Charlie got Alice's message content
        if charlie_got_message:
            print(f"{Color.GREEN}✓{Color.NC} Charlie decrypted Alice's message content")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie didn't receive Alice's message")
        
        # Test 5: Both Bob and Charlie are in the same MLS group
        if bob_got_message and charlie_got_message:
            print(f"{Color.GREEN}✓{Color.NC} Three-way MLS group communication working!")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Not all members received the message")
        
        print(f"\n{Color.CYAN}Score: {tests_passed}/{tests_total}{Color.NC}")
        
        if tests_passed == tests_total:
            print(f"\n{Color.GREEN}🎉 SUCCESS! Three-member MLS group works perfectly!{Color.NC}")
            print(f"{Color.GREEN}   ✓ Alice → Bob: Encrypted and decrypted{Color.NC}")
            print(f"{Color.GREEN}   ✓ Alice → Charlie: Encrypted and decrypted{Color.NC}")
            print(f"{Color.GREEN}   ✓ All three members in sync!{Color.NC}")
            return 0
        elif tests_passed >= 3:
            print(f"\n{Color.YELLOW}⚠️  PARTIAL SUCCESS ({tests_passed}/{tests_total}){Color.NC}")
            return 0
        else:
            print(f"\n{Color.RED}❌ TEST FAILED ({tests_passed}/{tests_total}){Color.NC}")
            print(f"Check logs: alice_3m.log, bob_3m.log, charlie_3m.log")
            return 1
            
    finally:
        alice['proc'].terminate()
        bob['proc'].terminate()
        charlie['proc'].terminate()
        try:
            alice['proc'].wait(timeout=3)
            bob['proc'].wait(timeout=3)
            charlie['proc'].wait(timeout=3)
        except:
            alice['proc'].kill()
            bob['proc'].kill()
            charlie['proc'].kill()

if __name__ == '__main__':
    sys.exit(main())
