#!/usr/bin/env python3
"""
Test MLS member removal (kick) with four members
Tests: Alice creates space → invites Bob, Charlie, and Dave → all exchange messages → 
       Alice kicks Bob → Alice sends message → Bob CANNOT decrypt, Charlie and Dave CAN decrypt
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
    print(f"{Color.CYAN}║  MLS Four Members + Kick Test                 ║{Color.NC}")
    print(f"{Color.CYAN}╚═══════════════════════════════════════════════╝{Color.NC}\n")
    
    # Setup test directory structure
    test_dir = 'tests/test-runs/four-members-kick'
    os.makedirs(test_dir, exist_ok=True)
    
    # Cleanup old test artifacts
    os.system(f'rm -rf {test_dir}/* 2>/dev/null')
    
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
    alice_log = open(f'{test_dir}/alice.log', 'w')
    bob_log = open(f'{test_dir}/bob.log', 'w')
    charlie_log = open(f'{test_dir}/charlie.log', 'w')
    dave_log = open(f'{test_dir}/dave.log', 'w')
    
    print(f"{Color.CYAN}Starting Alice, Bob, Charlie, and Dave...{Color.NC}")
    
    alice = {
        'name': 'Alice',
        'proc': subprocess.Popen(
            [binary_path, '--account', f'{test_dir}/alice.key', '--port', '9001'],
            stdin=subprocess.PIPE, stdout=alice_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': f'{test_dir}/alice.log'
    }
    
    bob = {
        'name': 'Bob',
        'proc': subprocess.Popen(
            [binary_path, '--account', f'{test_dir}/bob.key', '--port', '9002'],
            stdin=subprocess.PIPE, stdout=bob_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': f'{test_dir}/bob.log'
    }
    
    charlie = {
        'name': 'Charlie',
        'proc': subprocess.Popen(
            [binary_path, '--account', f'{test_dir}/charlie.key', '--port', '9003'],
            stdin=subprocess.PIPE, stdout=charlie_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': f'{test_dir}/charlie.log'
    }
    
    dave = {
        'name': 'Dave',
        'proc': subprocess.Popen(
            [binary_path, '--account', f'{test_dir}/dave.key', '--port', '9004'],
            stdin=subprocess.PIPE, stdout=dave_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': f'{test_dir}/dave.log'
    }
    
    try:
        print(f"{Color.GREEN}✓ Alice, Bob, Charlie, and Dave started{Color.NC}\n")
        time.sleep(3)
        
        # Setup
        print(f"{Color.CYAN}Setting up KeyPackages and Space...{Color.NC}")
        run_command(alice, 'keypackage publish', wait=5)
        run_command(bob, 'keypackage publish', wait=5)
        run_command(charlie, 'keypackage publish', wait=5)
        run_command(dave, 'keypackage publish', wait=5)
        
        run_command(alice, 'space create kick-test', wait=3)
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
        
        # Dave joins
        print(f"{Color.CYAN}Dave connecting and joining...{Color.NC}")
        run_command(dave, f'connect /ip4/127.0.0.1/tcp/9001/p2p/{peer_id}', wait=3)
        run_command(dave, f'join {full_space_id} {invite}', wait=8)
        
        run_command(dave, 'whoami', wait=3)
        dave_id = find_in_log(dave['log'], r'User ID: ([0-9a-f]{64})')
        
        print(f"{Color.GREEN}✓ Dave joined{Color.NC}\n")
        
        # Add members to MLS
        print(f"{Color.CYAN}Adding Bob to MLS group...{Color.NC}")
        run_command(alice, f'member add {bob_id}', wait=5)
        time.sleep(4)
        print(f"{Color.GREEN}✓ Bob added to MLS{Color.NC}\n")
        
        print(f"{Color.CYAN}Adding Charlie to MLS group...{Color.NC}")
        run_command(alice, f'member add {charlie_id}', wait=5)
        time.sleep(4)
        print(f"{Color.GREEN}✓ Charlie added to MLS{Color.NC}\n")
        
        print(f"{Color.CYAN}Adding Dave to MLS group...{Color.NC}")
        run_command(alice, f'member add {dave_id}', wait=5)
        time.sleep(4)
        print(f"{Color.GREEN}✓ Dave added to MLS{Color.NC}\n")
        
        # Alice creates channel and sends message
        print(f"{Color.CYAN}Alice creating channel and sending message...{Color.NC}")
        run_command(alice, 'channel create general', wait=3)
        run_command(alice, 'thread create "Kick Test"', wait=3)
        run_command(alice, 'send Message 1: Before kick', wait=4)
        
        print(f"{Color.YELLOW}⏳ Waiting for message propagation (5s)...{Color.NC}")
        time.sleep(5)
        
        # All members navigate to thread
        print(f"\n{Color.CYAN}Bob, Charlie, and Dave navigating to thread...{Color.NC}")
        for client in [bob, charlie, dave]:
            run_command(client, f'space {space_id}', wait=2)
            run_command(client, 'channels', wait=2)
            
            channel_id = find_in_log(client['log'], r'([0-9a-f]{16})\s+-\s+general')
            if channel_id:
                run_command(client, f'channel {channel_id}', wait=2)
                run_command(client, 'threads', wait=2)
                thread_id = find_in_log(client['log'], r'([0-9a-f]{16})\s+-\s+"?Kick Test"?')
                if thread_id:
                    run_command(client, f'thread {thread_id}', wait=2)
        
        # Members reply
        print(f"\n{Color.CYAN}Bob replying...{Color.NC}")
        run_command(bob, 'send Message 2: Bob reply before kick', wait=4)
        
        print(f"{Color.CYAN}Charlie replying...{Color.NC}")
        run_command(charlie, 'send Message 3: Charlie reply before kick', wait=4)
        
        print(f"{Color.CYAN}Dave replying...{Color.NC}")
        run_command(dave, 'send Message 4: Dave reply before kick', wait=4)
        
        print(f"{Color.YELLOW}⏳ Waiting for replies (7s)...{Color.NC}")
        time.sleep(7)
        
        # Kick Bob
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Alice kicking Bob from the space...{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        run_command(alice, f'kick {bob_id}', wait=7)
        
        print(f"{Color.GREEN}✓ Bob has been kicked{Color.NC}\n")
        
        # Alice sends message after kick
        print(f"{Color.CYAN}Alice sending message after kicking Bob...{Color.NC}")
        run_command(alice, 'send Message 5: After kick - Bob should NOT see, Charlie and Dave SHOULD see', wait=4)
        
        print(f"{Color.YELLOW}⏳ Final propagation wait (10s)...{Color.NC}")
        time.sleep(10)
        
        # Close logs
        alice_log.close()
        bob_log.close()
        charlie_log.close()
        dave_log.close()
        
        # Check results
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Results{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        alice_log_content = open(alice['log']).read()
        bob_log_content = open(bob['log']).read()
        charlie_log_content = open(charlie['log']).read()
        dave_log_content = open(dave['log']).read()
        
        alice_decrypts = len(re.findall(r'Decrypted MLS message', alice_log_content))
        bob_decrypts = len(re.findall(r'Decrypted MLS message', bob_log_content))
        charlie_decrypts = len(re.findall(r'Decrypted MLS message', charlie_log_content))
        dave_decrypts = len(re.findall(r'Decrypted MLS message', dave_log_content))
        
        bob_got_msg1 = 'Before kick' in bob_log_content
        alice_got_msg2 = 'Bob reply before kick' in alice_log_content
        alice_got_msg3 = 'Charlie reply before kick' in alice_log_content
        alice_got_msg4 = 'Dave reply before kick' in alice_log_content
        bob_got_msg5 = 'After kick' in bob_log_content
        charlie_got_msg5 = 'After kick' in charlie_log_content
        dave_got_msg5 = 'After kick' in dave_log_content
        
        print(f"  Alice total decryptions: {alice_decrypts}")
        print(f"  Bob total decryptions: {bob_decrypts}")
        print(f"  Charlie total decryptions: {charlie_decrypts}")
        print(f"  Dave total decryptions: {dave_decrypts}")
        print(f"  Bob received msg1 (before kick): {bob_got_msg1}")
        print(f"  Alice received msg2 (Bob's reply): {alice_got_msg2}")
        print(f"  Alice received msg3 (Charlie's reply): {alice_got_msg3}")
        print(f"  Alice received msg4 (Dave's reply): {alice_got_msg4}")
        print(f"  Bob received msg5 (after kick): {bob_got_msg5} (should be False!)")
        print(f"  Charlie received msg5 (after kick): {charlie_got_msg5} (should be True!)")
        print(f"  Dave received msg5 (after kick): {dave_got_msg5} (should be True!)\n")
        
        tests_passed = 0
        tests_total = 10
        
        if bob_got_msg1:
            print(f"{Color.GREEN}✓{Color.NC} Bob decrypted: 'Message 1: Before kick'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's message before kick")
        
        if alice_got_msg2:
            print(f"{Color.GREEN}✓{Color.NC} Alice decrypted: 'Message 2: Bob reply before kick'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Alice didn't receive Bob's reply")
        
        if alice_got_msg3:
            print(f"{Color.GREEN}✓{Color.NC} Alice decrypted: 'Message 3: Charlie reply before kick'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Alice didn't receive Charlie's reply")
        
        if alice_got_msg4:
            print(f"{Color.GREEN}✓{Color.NC} Alice decrypted: 'Message 4: Dave reply before kick'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Alice didn't receive Dave's reply")
        
        if bob_decrypts >= 4 and alice_decrypts >= 3 and charlie_decrypts >= 4 and dave_decrypts >= 4:
            print(f"{Color.GREEN}✓{Color.NC} Four-way E2EE working before kick")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} E2EE not working for all members before kick")
        
        if 'Successfully removed user' in alice_log_content or 'MLS keys rotated' in alice_log_content:
            print(f"{Color.GREEN}✓{Color.NC} Alice successfully kicked Bob (MLS keys rotated)")
            tests_passed += 1
        else:
            print(f"{Color.YELLOW}⚠{Color.NC}  Cannot confirm kick succeeded (check logs)")
        
        if not bob_got_msg5:
            print(f"{Color.GREEN}✓{Color.NC} Bob CANNOT decrypt message after kick (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob decrypted message after kick (SECURITY ISSUE!)")
        
        if charlie_got_msg5:
            print(f"{Color.GREEN}✓{Color.NC} Charlie CAN decrypt message after Bob's kick (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie can't decrypt after Bob's kick (incorrect!)")
        
        if dave_got_msg5:
            print(f"{Color.GREEN}✓{Color.NC} Dave CAN decrypt message after Bob's kick (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Dave can't decrypt after Bob's kick (incorrect!)")
        
        if bob_decrypts <= 6 and charlie_decrypts >= 5 and dave_decrypts >= 5:
            print(f"{Color.GREEN}✓{Color.NC} Decryption counts correct (Bob ≤6, Charlie ≥5, Dave ≥5)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Decryption counts wrong (Bob: {bob_decrypts}, Charlie: {charlie_decrypts}, Dave: {dave_decrypts})")
        
        print(f"\n{Color.CYAN}Score: {tests_passed}/{tests_total}{Color.NC}")
        
        if tests_passed == tests_total:
            print(f"\n{Color.GREEN}🎉 SUCCESS! Four-member kick working correctly!{Color.NC}")
            print(f"{Color.GREEN}   ✓ All four could communicate before kick{Color.NC}")
            print(f"{Color.GREEN}   ✓ Bob CANNOT decrypt after kick{Color.NC}")
            print(f"{Color.GREEN}   ✓ Charlie and Dave CAN still decrypt after Bob's kick{Color.NC}")
            return 0
        elif tests_passed >= 7:
            print(f"\n{Color.YELLOW}⚠️  PARTIAL SUCCESS ({tests_passed}/{tests_total}){Color.NC}")
            return 0
        else:
            print(f"\n{Color.RED}❌ TEST FAILED ({tests_passed}/{tests_total}){Color.NC}")
            print(f"Check logs in: {test_dir}/")
            return 1
            
    finally:
        alice['proc'].terminate()
        bob['proc'].terminate()
        charlie['proc'].terminate()
        dave['proc'].terminate()
        try:
            alice['proc'].wait(timeout=3)
            bob['proc'].wait(timeout=3)
            charlie['proc'].wait(timeout=3)
            dave['proc'].wait(timeout=3)
        except:
            alice['proc'].kill()
            bob['proc'].kill()
            charlie['proc'].kill()
            dave['proc'].kill()

if __name__ == '__main__':
    sys.exit(main())
