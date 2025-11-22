#!/usr/bin/env python3
"""
Automated Testing Framework for Spaceway MLS E2EE
This script automates the complete Alice & Bob workflow and captures all output for analysis.
"""

import subprocess
import time
import re
import os
import signal
import sys
from pathlib import Path

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
        """Start the Spaceway client"""
        print(f"{Color.BLUE}Starting {self.name} (port {self.port})...{Color.NC}")
        
        self.log_handle = open(self.log_file, 'w')
        
        cmd = [
            './target/release/spaceway',
            '--account', self.account,
            '--port', str(self.port)
        ]
        
        self.process = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=self.log_handle,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1
        )
        
        time.sleep(3)  # Wait for startup
        print(f"{Color.GREEN}{self.name} started (PID: {self.process.pid}){Color.NC}")
        
    def send_command(self, command, wait=2):
        """Send a command to the client"""
        print(f"{Color.YELLOW}[{self.name}]{Color.NC} > {command}")
        self.process.stdin.write(command + '\n')
        self.process.stdin.flush()
        time.sleep(wait)
        
    def stop(self):
        """Stop the client"""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
        if self.log_handle:
            self.log_handle.close()
            
    def read_log(self):
        """Read the entire log file"""
        with open(self.log_file, 'r') as f:
            return f.read()
            
    def find_in_log(self, pattern):
        """Find a pattern in the log and return the match"""
        log = self.read_log()
        match = re.search(pattern, log)
        return match.group(1) if match else None
        
    def check_log(self, pattern):
        """Check if a pattern exists in the log"""
        log = self.read_log()
        return bool(re.search(pattern, log))

class TestRunner:
    def __init__(self):
        self.alice = SpacewayClient('Alice', 'alice.key', 9001, 'alice_output.log')
        self.bob = SpacewayClient('Bob', 'bob.key', 9002, 'bob_output.log')
        self.results_file = 'test_results.txt'
        self.tests_passed = 0
        self.tests_total = 0
        self.results = []
        
    def log_result(self, message):
        """Log a result message"""
        print(message)
        self.results.append(message.replace(Color.GREEN, '').replace(Color.RED, '').replace(Color.YELLOW, '').replace(Color.NC, ''))
        
    def test(self, description, check_fn):
        """Run a test and log the result"""
        self.tests_total += 1
        try:
            if check_fn():
                self.log_result(f"{Color.GREEN}✓{Color.NC} {description}")
                self.tests_passed += 1
                return True
            else:
                self.log_result(f"{Color.RED}✗{Color.NC} {description}")
                return False
        except Exception as e:
            self.log_result(f"{Color.RED}✗{Color.NC} {description} (Error: {e})")
            return False
            
    def cleanup(self):
        """Clean up processes and temp files"""
        print(f"\n{Color.YELLOW}Cleaning up...{Color.NC}")
        self.alice.stop()
        self.bob.stop()
        
    def save_results(self):
        """Save results to file"""
        with open(self.results_file, 'w') as f:
            f.write('\n'.join(self.results))
            f.write(f'\n\nResults: {self.tests_passed}/{self.tests_total} tests passed\n')

if __name__ == '__main__':
    print("Test script created successfully")
