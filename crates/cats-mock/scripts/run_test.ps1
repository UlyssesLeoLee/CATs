$ErrorActionPreference = 'Continue'
Set-Location D:/CATs
$proc = Start-Process -FilePath "cargo.exe" -ArgumentList "test","-p","cats-mock","--no-fail-fast" `
    -RedirectStandardOutput "D:/CATs/test_out.log" `
    -RedirectStandardError "D:/CATs/test_err.log" `
    -NoNewWindow -PassThru
Write-Host "PID=$($proc.Id)"
$proc.WaitForExit(600)
Write-Host "EXIT=$($proc.ExitCode)"
