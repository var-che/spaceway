@echo off
REM Descord Interactive Demo
REM This shows a step-by-step walkthrough of Alice and Bob

echo.
echo ========================================
echo    DESCORD CLI INTERACTIVE DEMO
echo ========================================
echo.

REM Clean up old demo files
if exist demo-alice.key del demo-alice.key
if exist demo-bob.key del demo-bob.key
if exist alice-commands.txt del alice-commands.txt
if exist bob-commands.txt del bob-commands.txt

echo Creating command scripts...
echo.

REM Create Alice's commands
(
echo space create Tech Community
echo channel create general
echo thread create Introductions
echo send Welcome to Tech Community! This is a decentralized forum.
echo send Feel free to introduce yourself!
echo invite create
echo invite list
echo messages
) > alice-commands.txt

REM Create Bob's commands (simpler)
(
echo help
echo spaces
) > bob-commands.txt

echo =========================================
echo ALICE'S SESSION - Creating the community
echo =========================================
echo.
echo Press any key to start Alice's session...
pause > nul

echo Starting Alice...
echo.
start "ALICE - Descord" cmd /k ".\target\release\descord.exe --account demo-alice.key"

timeout /t 5

echo.
echo =========================================
echo BOB'S SESSION - Joining the community
echo =========================================
echo.
echo Press any key to start Bob's session in a new window...
pause > nul

echo Starting Bob...
echo.
start "BOB - Descord" cmd /k ".\target\release\descord.exe --account demo-bob.key"

echo.
echo =========================================
echo DEMO INSTRUCTIONS
echo =========================================
echo.
echo TWO WINDOWS ARE NOW OPEN:
echo   1. ALICE's window (started first)
echo   2. BOB's window (just opened)
echo.
echo MANUAL STEPS TO DEMONSTRATE:
echo.
echo IN ALICE'S WINDOW:
echo   1. Type: space create Tech Community
echo   2. Type: channel create general
echo   3. Type: thread create Introductions
echo   4. Type: send Welcome to the community!
echo   5. Type: invite create
echo   6. Type: invite list
echo      (Copy the invite code that appears)
echo.
echo IN BOB'S WINDOW:
echo   1. Type: join [paste Alice's invite code]
echo   2. Type: messages
echo      (Bob should see Alice's welcome message!)
echo   3. Type: send Hi Alice, thanks for the invite!
echo.
echo BACK IN ALICE'S WINDOW:
echo   1. Type: refresh
echo   2. Type: messages
echo      (Alice should see Bob's message!)
echo.
echo =========================================
echo.
echo Press any key to close this demo script...
echo (The ALICE and BOB windows will remain open)
pause > nul
