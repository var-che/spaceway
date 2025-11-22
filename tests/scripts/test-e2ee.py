#!/usr/bin/env python3
"""
Spaceway MLS E2EE Bidirectional Messaging Test
Tests that both Alice and Bob can send and decrypt each other's encrypted messages.
"""

import subprocess
import time
import re
import os

class Color:
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[1;33m'
    BLUE = '\033[0;34m'
    CYAN = '\033[0;36m'
    NC = '\033[0m'

class SpacewayClient:
    def __init__(self, name, account, port, log_file):
        self.name = name
        self.account = account
        self.port = port
        self.log_file = log_file
        self.process = None
        self.log_handle = None
        
    def start(self):
        print(f"{Color.BLUE}Starting {self.name} (port {self.port})...{Color.NC}")
        self.log_handle = open(self.log_file, 'w')
        cmd = ['./target/release/spaceway', '--account', self.account, '--port', str(self.port)]
        self.process = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=self.log_handle, 
                                       stderr=subprocess.STDOUT, text=True, bufsize=1)
        time.sleep(3)
        print(f"{Color.GREEN}{self.name} started (PID: {self.process.pid}){Color.NC}")
        
    def send_command(self, command, wait=2):
        print(f"{Color.YELLOW}[{self.name}]{Color.NC} > {command}")
        self.process.stdin.write(command + '\n')
        self.process.stdin.flush()
        time.sleep(wait)
        
    def stop(self):
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
        if self.log_handle:
            self.log_handle.close()
            
    def read_log(self):
        with open(self.log_file, 'r') as f:
            return f.read()
            
    def find_in_log(self, pattern):
        log = self.read_log()
        match = re.search(pattern, log)
        return match.group(1) if match else None
        
    def check_log(self, pattern):
        return bool(re.search(pattern, self.read_log()))

def main():
    # Cleanup
    print(f"{Color.BLUE}Cleaning old test data...{Color.NC}")
    os.system('rm -rf *-data/ *.key *.history alice_e2ee.log bob_e2ee.log 2>/dev/null')
    
    # Build
    print(f"\n{Color.BLUE}Building Spaceway...{Color.NC}")
    if os.system('cargo +nightly build --release 2>&1 | tail -3') != 0:
        print(f"{Color.RED}Build failed!{Color.NC}")
        return
    
    alice = SpacewayClient('Alice', 'alice.key', 9001, 'alice_e2ee.log')
    bob = SpacewayClient('Bob', 'bob.key', 9002, 'bob_e2ee.log')
    
    try:
        alice.start()
        bob.start()
        time.sleep(2)
        
        # Setup
        alice.send_command('keypackage publish', wait=5)
        bob.send_command('keypackage publish', wait=5)
        
        alice.send_command('space create bidirectional-test', wait=3)
        space_id = alice.find_in_log(r'Created space: .+? \(([0-9a-f]{16})\)')
        alice.send_command('context', wait=2)
        full_space_id = alice.find_in_log(r'Space: ([0-9a-f]{64})')
        
        alice.send_command('invite create', wait=3)
        invite = alice.find_in_log(r'Created invite code: (\w+)')
        
        alice.send_command('network', wait=2)
        peer_id = alice.find_in_log(r'Peer ID: (\w+)')
        
        bob.send_command(f'connect /ip4/127.0.0.1/tcp/9001/p2p/{peer_id}', wait=3)
        bob.send_command(f'join {full_space_id} {invite}', wait=5)
        
        bob.send_command('whoami', wait=2)
        bob_id = bob.find_in_log(r'User ID: ([0-9a-f]{64})')
        
        alice.send_command(f'member add {bob_id}', wait=5)
        time.sleep(3)
        
        # Alice sends first message
        alice.send_command('channel create general', wait=3)
        alice.send_command('thread create "E2EE Test"', wait=3)
        alice.send_command('send Hello Bob! Can you read this encrypted message?', wait=4)
        time.sleep(3)
        
        # Bob navigates and sends reply
        bob.send_command('space list', wait=2)
        bob.send_command(f'space {space_id}', wait=2)
        bob.send_command('channels', wait=2)
        channel_id = bob.find_in_log(r'([0-9a-f]{16})\s+-\s+general')
        if channel_id:
            bob.send_command(f'channel {channel_id}', wait=2)
            bob.send_command('threads', wait=2)
            thread_id = bob.find_in_log(r'([0-9a-f]{16})\s+-\s+"?E2EE Test"?')
            if thread_id:
                bob.send_command(f'thread {thread_id}', wait=2)
        
        bob.send_command('send Yes Alice! I can decrypt and reply with encryption!', wait=4)
        time.sleep(3)
        
        # Alice sends another message
        alice.send_command('send Perfect! Bidirectional E2EE is working!', wait=4)
        time.sleep(3)
        
        # Check results
        print(f"\n{Color.BLUE}{'='*60}{Color.NC}")
        print(f"{Color.BLUE}Test Results{Color.NC}")
        print(f"{Color.BLUE}{'='*60}{Color.NC}\n")
        
        alice_decrypts = len(re.findall(r'Decrypted MLS message', alice.read_log()))
        bob_decrypts = len(re.findall(r'Decrypted MLS message', bob.read_log()))
        
        bob_got_hello = bob.check_log(r'Can you read this encrypted message')
        alice_got_reply = alice.check_log(r'I can decrypt and reply')
        bob_got_perfect = bob.check_log(r'Bidirectional E2EE is working')
        
        print(f"{Color.CYAN}Alice decryptions:{Color.NC} {alice_decrypts}")
        print(f"{Color.CYAN}Bob decryptions:{Color.NC} {bob_decrypts}")
        print()
        
        if bob_decrypts >= 3:
            print(f"{Color.GREEN}✓{Color.NC} Bob decrypted Alice's messages ({bob_decrypts} total)")
        else:
            print(f"{Color.RED}✗{Color.NC} Bob should have decrypted at least 3 messages")
            
        if bob_got_hello:
            print(f"{Color.GREEN}✓{Color.NC} Bob received: 'Can you read this encrypted message?'")
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's first message")
            
        if alice_decrypts >= 1:
            print(f"{Color.GREEN}✓{Color.NC} Alice decrypted Bob's messages ({alice_decrypts} total)")
        else:
            print(f"{Color.RED}✗{Color.NC} Alice should have decrypted at least 1 message")
            
        if alice_got_reply:
            print(f"{Color.GREEN}✓{Color.NC} Alice received: 'I can decrypt and reply'")
        else:
            print(f"{Color.RED}✗{Color.NC} Alice didn't receive Bob's reply")
            
        if bob_got_perfect:
            print(f"{Color.GREEN}✓{Color.NC} Bob received: 'Bidirectional E2EE is working'")
        else:
            print(f"{Color.RED}✗{Color.NC} Bob didn't receive Alice's second message")
        
        print(f"\n{Color.BLUE}Logs saved to: alice_e2ee.log, bob_e2ee.log{Color.NC}\n")
        
        if alice_decrypts >= 1 and bob_decrypts >= 3 and bob_got_hello and alice_got_reply:
            print(f"{Color.GREEN}🎉 SUCCESS! Bidirectional E2EE messaging works!{Color.NC}\n")
        else:
            print(f"{Color.YELLOW}⚠ Some tests failed - check logs for details{Color.NC}\n")
            
    finally:
        alice.stop()
        bob.stop()

if __name__ == '__main__':
    main()
