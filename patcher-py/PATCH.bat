@echo off
cd /d "%~dp0.."
where py >nul 2>nul && (py -3 "%~dp0zoids_combo_patch.py" %*) || (python "%~dp0zoids_combo_patch.py" %*)
pause
