#!/usr/bin/env python3
"""
Test bidirectional E2EE messaging between Alice and Bob
Focuses on ensuring both can encrypt and decrypt each other's messages
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
    print(f"{Color.CYAN}║  Bidirectional E2EE Messaging Test            ║{Color.NC}")
    print(f"{Color.CYAN}╚═══════════════════════════════════════════════╝{Color.NC}\n")
    
    # Cleanup
    os.system('rm -rf *-data/ *.key *.history alice_bidir.log bob_bidir.log 2>/dev/null')
    
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
    alice_log = open('alice_bidir.log', 'w')
    bob_log = open('bob_bidir.log', 'w')
    
    print(f"{Color.CYAN}Starting Alice and Bob...{Color.NC}")
    
    alice = {
        'name': 'Alice',
        'proc': subprocess.Popen(
            [binary_path, '--account', 'alice.key', '--port', '9001'],
            stdin=subprocess.PIPE, stdout=alice_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': 'alice_bidir.log'
    }
    
    bob = {
        'name': 'Bob',
        'proc': subprocess.Popen(
            [binary_path, '--account', 'bob.key', '--port', '9002'],
            stdin=subprocess.PIPE, stdout=bob_log, stderr=subprocess.STDOUT, text=True, bufsize=1
        ),
        'log': 'bob_bidir.log'
    }
    
    try:
        print(f"{Color.GREEN}✓ Alice and Bob started{Color.NC}\n")
        time.sleep(3)
        
        # Setup
        print(f"{Color.CYAN}Setting up KeyPackages and Space...{Color.NC}")
        run_command(alice, 'keypackage publish', wait=5)
        run_command(bob, 'keypackage publish', wait=5)
        
        run_command(alice, 'space create bidir-test', wait=3)
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
        
        # Add Bob to MLS
        print(f"{Color.CYAN}Adding Bob to MLS group...{Color.NC}")
        run_command(alice, f'member add {bob_id}', wait=5)
        time.sleep(4)  # Wait for Welcome message
        print(f"{Color.GREEN}✓ Bob added to MLS{Color.NC}\n")
        
        # Alice sends first message
        print(f"{Color.CYAN}Alice creating channel and sending message...{Color.NC}")
        run_command(alice, 'channel create general', wait=3)
        run_command(alice, 'thread create "Bidir Test"', wait=3)
        run_command(alice, 'send Hello Bob! Can you decrypt this?', wait=4)
        
        # Important: Wait for GossipSub propagation
        print(f"{Color.YELLOW}⏳ Waiting for message propagation (5s)...{Color.NC}")
        time.sleep(5)
        
        # Bob navigates and replies
        print(f"\n{Color.CYAN}Bob navigating and replying...{Color.NC}")
        run_command(bob, f'space {space_id}', wait=2)
        run_command(bob, 'channels', wait=2)
        
        channel_id = find_in_log(bob['log'], r'([0-9a-f]{16})\s+-\s+general')
        if channel_id:
            run_command(bob, f'channel {channel_id}', wait=2)
            run_command(bob, 'threads', wait=2)
            thread_id = find_in_log(bob['log'], r'([0-9a-f]{16})\s+-\s+"?Bidir Test"?')
            if thread_id:
                run_command(bob, f'thread {thread_id}', wait=2)
        
        run_command(bob, 'send Yes Alice! I can decrypt AND send encrypted messages!', wait=4)
        
        # CRITICAL: Wait for Bob's message to propagate to Alice
        print(f"{Color.YELLOW}⏳ Waiting for Bob's message to reach Alice (7s)...{Color.NC}")
        time.sleep(7)
        
        # Alice sends final message
        run_command(alice, 'send Perfect! Bidirectional E2EE confirmed!', wait=4)
        
        # CRITICAL: Final wait for ALL messages to propagate through GossipSub
        # This ensures Alice receives Bob's message and Bob receives Alice's second message
        print(f"{Color.YELLOW}⏳ Final propagation wait (10s) - ensuring all messages delivered...{Color.NC}")
        time.sleep(10)
        
        # Close logs
        alice_log.close()
        bob_log.close()
        
        # Check results
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Results{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        alice_decrypts = len(re.findall(r'Decrypted MLS message', open(alice['log']).read()))
        bob_decrypts = len(re.findall(r'Decrypted MLS message', open(bob['log']).read()))
        
        bob_got_hello = check_log(bob['log'], r'Can you decrypt this')
        alice_got_reply = check_log(alice['log'], r'I can decrypt AND send')
        bob_got_perfect = check_log(bob['log'], r'Bidirectional E2EE confirmed')
        
        print(f"  Alice decryptions: {alice_decrypts}")
        print(f"  Bob decryptions: {bob_decrypts}\n")
        
        tests_passed = 0
        tests_total = 5
        
        if bob_decrypts >= 3:
            print(f"{Color.GREEN}✓{Color.NC} Bob decrypted Alice's messages")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob should have ≥3 decryptions, got {bob_decrypts}")
        
        if bob_got_hello:
            print(f"{Color.GREEN}✓{Color.NC} Bob received: 'Can you decrypt this'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's first message")
        
        if alice_decrypts >= 1:
            print(f"{Color.GREEN}✓{Color.NC} Alice decrypted Bob's message")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Alice should have ≥1 decryption, got {alice_decrypts}")
        
        if alice_got_reply:
            print(f"{Color.GREEN}✓{Color.NC} Alice received: 'I can decrypt AND send'")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Alice didn't receive Bob's message")
        
        if bob_got_perfect:
            print(f"{Color.GREEN}✓{Color.NC} Bob received Alice's second message")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's second message")
        
        print(f"\n{Color.CYAN}Score: {tests_passed}/{tests_total}{Color.NC}")
        
        if tests_passed == tests_total:
            print(f"\n{Color.GREEN}🎉 SUCCESS! Bidirectional E2EE works perfectly!{Color.NC}")
            return 0
        else:
            print(f"\n{Color.YELLOW}⚠ Some tests failed{Color.NC}")
            print(f"Check logs: alice_bidir.log, bob_bidir.log")
            return 1
            
    finally:
        alice['proc'].terminate()
        bob['proc'].terminate()
        try:
            alice['proc'].wait(timeout=3)
            bob['proc'].wait(timeout=3)
        except:
            alice['proc'].kill()
            bob['proc'].kill()

if __name__ == '__main__':
    sys.exit(main())
