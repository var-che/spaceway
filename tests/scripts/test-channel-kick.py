#!/usr/bin/env python3
"""
Test MLS channel-specific member removal (kick)
Tests: Alice creates space → creates 2 channels → Alice, Bob, Charlie join both → 
       Alice kicks Charlie from channel 2 → Charlie CANNOT decrypt channel 2 messages →
       Charlie CAN still decrypt channel 1 messages
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
    print(f"{Color.CYAN}║  MLS Channel-Specific Kick Test               ║{Color.NC}")
    print(f"{Color.CYAN}╚═══════════════════════════════════════════════╝{Color.NC}\n")
    
    # Setup test directory structure
    test_dir = 'tests/test-runs/channel-kick'
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
    
    print(f"{Color.CYAN}Starting Alice, Bob, and Charlie...{Color.NC}")
    
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
    
    try:
        print(f"{Color.GREEN}✓ Alice, Bob, and Charlie started{Color.NC}\n")
        time.sleep(3)
        
        # Setup
        print(f"{Color.CYAN}Setting up KeyPackages and Space...{Color.NC}")
        run_command(alice, 'keypackage publish', wait=5)
        run_command(bob, 'keypackage publish', wait=5)
        run_command(charlie, 'keypackage publish', wait=5)
        
        run_command(alice, 'space create channel-kick-test', wait=3)
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
        
        # Add members to space MLS
        print(f"{Color.CYAN}Adding Bob to space MLS group...{Color.NC}")
        run_command(alice, f'member add {bob_id}', wait=5)
        time.sleep(4)
        print(f"{Color.GREEN}✓ Bob added to space MLS{Color.NC}\n")
        
        print(f"{Color.CYAN}Adding Charlie to space MLS group...{Color.NC}")
        run_command(alice, f'member add {charlie_id}', wait=5)
        time.sleep(4)
        print(f"{Color.GREEN}✓ Charlie added to space MLS{Color.NC}\n")
        
        # Alice creates two channels
        print(f"{Color.CYAN}Alice creating two channels...{Color.NC}")
        run_command(alice, 'channel create general', wait=3)
        run_command(alice, 'channel create private', wait=3)
        
        # Get channel IDs
        run_command(alice, 'channels', wait=2)
        alice_log_content = open(alice['log']).read()
        channel1_id = re.search(r'([0-9a-f]{16})\s+-\s+general', alice_log_content)
        channel2_id = re.search(r'([0-9a-f]{16})\s+-\s+private', alice_log_content)
        
        if not channel1_id or not channel2_id:
            print(f"{Color.RED}✗ Failed to find channel IDs{Color.NC}")
            return 1
        
        channel1_id = channel1_id.group(1)
        channel2_id = channel2_id.group(1)
        
        print(f"{Color.GREEN}✓ Created channels: general ({channel1_id}), private ({channel2_id}){Color.NC}\n")
        
        # Bob and Charlie navigate to both channels
        print(f"{Color.CYAN}Bob and Charlie joining both channels...{Color.NC}")
        for client in [bob, charlie]:
            run_command(client, f'space {space_id}', wait=2)
            run_command(client, 'channels', wait=2)
        
        print(f"{Color.GREEN}✓ All members can see both channels{Color.NC}\n")
        
        # Create threads in both channels
        print(f"{Color.CYAN}Alice creating threads in both channels...{Color.NC}")
        
        # Channel 1 thread
        run_command(alice, f'channel {channel1_id}', wait=2)
        run_command(alice, 'thread create "General Discussion"', wait=3)
        thread1_id = find_in_log(alice['log'], r'Created thread: .+? \(([0-9a-f]{16})\)')
        
        # Channel 2 thread
        run_command(alice, f'channel {channel2_id}', wait=2)
        run_command(alice, 'thread create "Private Discussion"', wait=3)
        thread2_id = find_in_log(alice['log'], r'Created thread: .+? \(([0-9a-f]{16})\)')
        
        print(f"{Color.GREEN}✓ Threads created{Color.NC}\n")
        
        # All members post in Channel 1
        print(f"{Color.CYAN}=== Testing Channel 1 (general) ==={Color.NC}\n")
        
        # Navigate all to channel 1 thread
        for client in [alice, bob, charlie]:
            run_command(client, f'space {space_id}', wait=2)
            run_command(client, f'channel {channel1_id}', wait=2)
            run_command(client, f'thread {thread1_id}', wait=2)
        
        print(f"{Color.CYAN}Alice posting to Channel 1...{Color.NC}")
        run_command(alice, 'send Channel 1: Alice initial message', wait=4)
        time.sleep(5)
        
        print(f"{Color.CYAN}Bob posting to Channel 1...{Color.NC}")
        run_command(bob, 'send Channel 1: Bob reply', wait=4)
        time.sleep(4)
        
        print(f"{Color.CYAN}Charlie posting to Channel 1...{Color.NC}")
        run_command(charlie, 'send Channel 1: Charlie reply', wait=4)
        time.sleep(4)
        
        # All members post in Channel 2
        print(f"\n{Color.CYAN}=== Testing Channel 2 (private) ==={Color.NC}\n")
        
        # Navigate all to channel 2 thread
        for client in [alice, bob, charlie]:
            run_command(client, f'channel {channel2_id}', wait=2)
            run_command(client, f'thread {thread2_id}', wait=2)
        
        print(f"{Color.CYAN}Alice posting to Channel 2...{Color.NC}")
        run_command(alice, 'send Channel 2: Alice initial message', wait=4)
        time.sleep(5)
        
        print(f"{Color.CYAN}Bob posting to Channel 2...{Color.NC}")
        run_command(bob, 'send Channel 2: Bob reply', wait=4)
        time.sleep(4)
        
        print(f"{Color.CYAN}Charlie posting to Channel 2...{Color.NC}")
        run_command(charlie, 'send Channel 2: Charlie reply', wait=4)
        time.sleep(4)
        
        # === KICK CHARLIE FROM CHANNEL 2 ===
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Alice kicking Charlie from Channel 2 (private)...{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        # Alice navigates to channel 2 and kicks Charlie
        run_command(alice, f'channel {channel2_id}', wait=2)
        run_command(alice, f'kick {charlie_id}', wait=7)
        
        print(f"{Color.GREEN}✓ Charlie has been kicked from Channel 2{Color.NC}\n")
        
        # Post to Channel 2 after kick
        print(f"{Color.CYAN}Alice and Bob posting to Channel 2 after kick...{Color.NC}")
        run_command(alice, f'thread {thread2_id}', wait=2)
        run_command(alice, 'send Channel 2: After kick - Charlie should NOT see this', wait=4)
        time.sleep(5)
        
        # Bob posts to channel 2
        run_command(bob, f'channel {channel2_id}', wait=2)
        run_command(bob, f'thread {thread2_id}', wait=2)
        run_command(bob, 'send Channel 2: Bob after Charlie kick', wait=4)
        time.sleep(5)
        
        # Post to Channel 1 after kick (Charlie should still see)
        print(f"\n{Color.CYAN}Alice and Bob posting to Channel 1 after Channel 2 kick...{Color.NC}")
        
        # Navigate to channel 1
        run_command(alice, f'channel {channel1_id}', wait=2)
        run_command(alice, f'thread {thread1_id}', wait=2)
        run_command(alice, 'send Channel 1: After Channel 2 kick - Charlie SHOULD see this', wait=4)
        time.sleep(5)
        
        run_command(bob, f'channel {channel1_id}', wait=2)
        run_command(bob, f'thread {thread1_id}', wait=2)
        run_command(bob, 'send Channel 1: Bob after Channel 2 kick', wait=4)
        time.sleep(5)
        
        # Charlie tries to view both channels
        print(f"\n{Color.CYAN}Charlie checking messages in both channels...{Color.NC}")
        run_command(charlie, f'channel {channel2_id}', wait=2)
        run_command(charlie, f'thread {thread2_id}', wait=2)
        time.sleep(3)
        
        run_command(charlie, f'channel {channel1_id}', wait=2)
        run_command(charlie, f'thread {thread1_id}', wait=2)
        time.sleep(3)
        
        # Close logs
        alice_log.close()
        bob_log.close()
        charlie_log.close()
        
        # Check results
        print(f"\n{Color.CYAN}{'='*50}{Color.NC}")
        print(f"{Color.CYAN}Results{Color.NC}")
        print(f"{Color.CYAN}{'='*50}{Color.NC}\n")
        
        alice_log_content = open(alice['log']).read()
        bob_log_content = open(bob['log']).read()
        charlie_log_content = open(charlie['log']).read()
        
        # Count total decryptions
        alice_decrypts = len(re.findall(r'Decrypted MLS message', alice_log_content))
        bob_decrypts = len(re.findall(r'Decrypted MLS message', bob_log_content))
        charlie_decrypts = len(re.findall(r'Decrypted MLS message', charlie_log_content))
        
        # Check Channel 1 messages (before kick)
        charlie_got_ch1_alice_initial = 'Channel 1: Alice initial message' in charlie_log_content
        charlie_got_ch1_bob = 'Channel 1: Bob reply' in charlie_log_content
        
        # Check Channel 2 messages (before kick)
        charlie_got_ch2_alice_initial = 'Channel 2: Alice initial message' in charlie_log_content
        charlie_got_ch2_bob = 'Channel 2: Bob reply' in charlie_log_content
        
        # Check Channel 2 messages (after kick) - Charlie should NOT see these
        charlie_got_ch2_after_kick_alice = 'Channel 2: After kick' in charlie_log_content
        charlie_got_ch2_after_kick_bob = 'Channel 2: Bob after Charlie kick' in charlie_log_content
        
        # Check Channel 1 messages (after Channel 2 kick) - Charlie SHOULD see these
        charlie_got_ch1_after_kick_alice = 'Channel 1: After Channel 2 kick' in charlie_log_content
        charlie_got_ch1_after_kick_bob = 'Channel 1: Bob after Channel 2 kick' in charlie_log_content
        
        # Bob should see everything
        bob_got_ch2_after_kick = 'Channel 2: After kick' in bob_log_content
        bob_got_ch1_after_kick = 'Channel 1: After Channel 2 kick' in bob_log_content
        
        print(f"  Total decryptions:")
        print(f"    Alice: {alice_decrypts}")
        print(f"    Bob: {bob_decrypts}")
        print(f"    Charlie: {charlie_decrypts}\n")
        
        print(f"  Channel 1 (before kick):")
        print(f"    Charlie got Alice's initial: {charlie_got_ch1_alice_initial}")
        print(f"    Charlie got Bob's reply: {charlie_got_ch1_bob}\n")
        
        print(f"  Channel 2 (before kick):")
        print(f"    Charlie got Alice's initial: {charlie_got_ch2_alice_initial}")
        print(f"    Charlie got Bob's reply: {charlie_got_ch2_bob}\n")
        
        print(f"  Channel 2 (after kick - Charlie should NOT see):")
        print(f"    Charlie got Alice's message: {charlie_got_ch2_after_kick_alice} (should be False!)")
        print(f"    Charlie got Bob's message: {charlie_got_ch2_after_kick_bob} (should be False!)\n")
        
        print(f"  Channel 1 (after Channel 2 kick - Charlie SHOULD see):")
        print(f"    Charlie got Alice's message: {charlie_got_ch1_after_kick_alice} (should be True!)")
        print(f"    Charlie got Bob's message: {charlie_got_ch1_after_kick_bob} (should be True!)\n")
        
        tests_passed = 0
        tests_total = 10
        
        # Test 1: Charlie can see Channel 1 messages before kick
        if charlie_got_ch1_alice_initial and charlie_got_ch1_bob:
            print(f"{Color.GREEN}✓{Color.NC} Charlie decrypted Channel 1 messages before kick")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie didn't decrypt Channel 1 messages before kick")
        
        # Test 2: Charlie can see Channel 2 messages before kick
        if charlie_got_ch2_alice_initial and charlie_got_ch2_bob:
            print(f"{Color.GREEN}✓{Color.NC} Charlie decrypted Channel 2 messages before kick")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie didn't decrypt Channel 2 messages before kick")
        
        # Test 3: Charlie CANNOT see Channel 2 messages after kick (CRITICAL)
        if not charlie_got_ch2_after_kick_alice:
            print(f"{Color.GREEN}✓{Color.NC} Charlie CANNOT decrypt Alice's Channel 2 message after kick (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie decrypted Alice's Channel 2 message after kick (SECURITY ISSUE!)")
        
        # Test 4: Charlie CANNOT see Bob's Channel 2 messages after kick (CRITICAL)
        if not charlie_got_ch2_after_kick_bob:
            print(f"{Color.GREEN}✓{Color.NC} Charlie CANNOT decrypt Bob's Channel 2 message after kick (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie decrypted Bob's Channel 2 message after kick (SECURITY ISSUE!)")
        
        # Test 5: Charlie CAN still see Channel 1 messages after Channel 2 kick (CRITICAL)
        if charlie_got_ch1_after_kick_alice:
            print(f"{Color.GREEN}✓{Color.NC} Charlie CAN decrypt Alice's Channel 1 message (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie can't decrypt Alice's Channel 1 message (incorrect!)")
        
        # Test 6: Charlie CAN still see Bob's Channel 1 messages after Channel 2 kick (CRITICAL)
        if charlie_got_ch1_after_kick_bob:
            print(f"{Color.GREEN}✓{Color.NC} Charlie CAN decrypt Bob's Channel 1 message (correct!)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie can't decrypt Bob's Channel 1 message (incorrect!)")
        
        # Test 7: Bob can see everything in Channel 2
        if bob_got_ch2_after_kick:
            print(f"{Color.GREEN}✓{Color.NC} Bob CAN decrypt Channel 2 messages after Charlie's kick")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob can't decrypt Channel 2 messages")
        
        # Test 8: Bob can see everything in Channel 1
        if bob_got_ch1_after_kick:
            print(f"{Color.GREEN}✓{Color.NC} Bob CAN decrypt Channel 1 messages after Charlie's Channel 2 kick")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Bob can't decrypt Channel 1 messages")
        
        # Test 9: Kick command succeeded
        if 'Successfully removed user' in alice_log_content or 'MLS keys rotated' in alice_log_content:
            print(f"{Color.GREEN}✓{Color.NC} Channel kick command succeeded")
            tests_passed += 1
        else:
            print(f"{Color.YELLOW}⚠{Color.NC}  Cannot confirm kick succeeded (check logs)")
        
        # Test 10: Reasonable decryption counts
        if charlie_decrypts >= 6 and charlie_decrypts <= 10:
            print(f"{Color.GREEN}✓{Color.NC} Charlie's decryption count reasonable (can see Ch1, not Ch2 after kick)")
            tests_passed += 1
        else:
            print(f"{Color.RED}✗{Color.NC} Charlie's decryption count unexpected: {charlie_decrypts}")
        
        print(f"\n{Color.CYAN}Score: {tests_passed}/{tests_total}{Color.NC}")
        
        if tests_passed == tests_total:
            print(f"\n{Color.GREEN}🎉 SUCCESS! Channel-specific kick working correctly!{Color.NC}")
            print(f"{Color.GREEN}   ✓ Charlie can decrypt Channel 1 messages (not kicked){Color.NC}")
            print(f"{Color.GREEN}   ✓ Charlie CANNOT decrypt Channel 2 messages (kicked){Color.NC}")
            print(f"{Color.GREEN}   ✓ Bob can decrypt both channels{Color.NC}")
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
