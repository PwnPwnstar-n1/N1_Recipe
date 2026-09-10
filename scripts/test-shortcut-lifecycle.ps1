param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath,
    [ValidateRange(1, 100)]
    [int]$Cycles = 20
)

$ErrorActionPreference = "Stop"
$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path

$nativeSource = @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class N1ShortcutLifecycleNative {
    public delegate bool EnumProc(IntPtr handle, IntPtr state);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr state);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr handle, StringBuilder text, int maxCount);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out Rect rect);
    [DllImport("user32.dll")] public static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr handle, uint message, IntPtr word, IntPtr value);
    public struct Rect { public int Left, Top, Right, Bottom; }
}
'@
Add-Type -TypeDefinition $nativeSource

function Send-LauncherShortcut {
    [N1ShortcutLifecycleNative]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)
    [N1ShortcutLifecycleNative]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [N1ShortcutLifecycleNative]::keybd_event(0x20, 0, 0, [UIntPtr]::Zero)
    [N1ShortcutLifecycleNative]::keybd_event(0x20, 0, 2, [UIntPtr]::Zero)
    [N1ShortcutLifecycleNative]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [N1ShortcutLifecycleNative]::keybd_event(0x11, 0, 2, [UIntPtr]::Zero)
}

function Get-LauncherWindow([int]$ProcessId) {
    $windows = [System.Collections.Generic.List[object]]::new()
    [void][N1ShortcutLifecycleNative]::EnumWindows({
        param($handle, $state)
        $ownerId = 0
        [void][N1ShortcutLifecycleNative]::GetWindowThreadProcessId($handle, [ref]$ownerId)
        if ($ownerId -eq $ProcessId) {
            $title = [Text.StringBuilder]::new(256)
            [void][N1ShortcutLifecycleNative]::GetWindowText($handle, $title, 256)
            $rect = New-Object N1ShortcutLifecycleNative+Rect
            [void][N1ShortcutLifecycleNative]::GetWindowRect($handle, [ref]$rect)
            $width = $rect.Right - $rect.Left
            $height = $rect.Bottom - $rect.Top
            if ($title.ToString() -eq "N1_Recipe" -and $width -gt 300) {
                $windows.Add([pscustomobject]@{ Handle = $handle; Area = $width * $height })
            }
        }
        return $true
    }, [IntPtr]::Zero)
    return $windows | Sort-Object Area | Select-Object -First 1
}

function Wait-WindowVisibility([IntPtr]$Handle, [bool]$Expected) {
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ([N1ShortcutLifecycleNative]::IsWindowVisible($Handle) -eq $Expected) {
            return $true
        }
        Start-Sleep -Milliseconds 50
    }
    return $false
}

$existing = Get-Process n1_recipe -ErrorAction SilentlyContinue
if ($existing) {
    throw "Close existing n1_recipe processes before running the lifecycle harness."
}

$appProcess = Start-Process -FilePath $resolvedExe -PassThru
try {
    # Window creation happens before tray and shortcut registration in setup.
    # Wait for the entire setup sequence so the harness cannot race registration.
    Start-Sleep -Milliseconds 1500
    $launcher = $null
    for ($attempt = 0; $attempt -lt 50 -and -not $launcher; $attempt++) {
        Start-Sleep -Milliseconds 100
        $launcher = Get-LauncherWindow $appProcess.Id
    }
    if (-not $launcher) { throw "Launcher window was not created during startup." }

    if ([N1ShortcutLifecycleNative]::IsWindowVisible($launcher.Handle)) {
        [void][N1ShortcutLifecycleNative]::SendMessage($launcher.Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        if (-not (Wait-WindowVisibility $launcher.Handle $false)) {
            throw "Could not normalize the launcher to hidden state."
        }
    }

    for ($iteration = 1; $iteration -le $Cycles; $iteration++) {
        Send-LauncherShortcut
        if (-not (Wait-WindowVisibility $launcher.Handle $true)) {
            throw "Cycle ${iteration}: shortcut did not show the launcher."
        }
        # Let set_focus and the corresponding WebView focus event drain before
        # simulating a user's close click.
        Start-Sleep -Milliseconds 150
        [void][N1ShortcutLifecycleNative]::SendMessage($launcher.Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        if (-not (Wait-WindowVisibility $launcher.Handle $false)) {
            throw "Cycle ${iteration}: close did not hide the launcher."
        }
        # Drain the focus-lost callback before the next shortcut press.
        Start-Sleep -Milliseconds 150
        if (-not (Get-Process -Id $appProcess.Id -ErrorAction SilentlyContinue)) {
            throw "Cycle ${iteration}: application exited unexpectedly."
        }
    }

    [pscustomobject]@{
        Result = "PASS"
        Cycles = $Cycles
        Executable = $resolvedExe
        LauncherHandle = $launcher.Handle.ToInt64()
        ProcessAlive = $true
    }
}
finally {
    if (Get-Process -Id $appProcess.Id -ErrorAction SilentlyContinue) {
        Stop-Process -Id $appProcess.Id
    }
}
