param(
    [Parameter(Mandatory=$true)][string]$ExePath,
    [ValidateSet('Stress','MouseOnly','ConflictRecovery')][string]$Mode='Stress',
    [ValidateRange(1,100)][int]$Cycles=100
)
$ErrorActionPreference='Stop'
$exe=(Resolve-Path -LiteralPath $ExePath).Path
$source=@'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class N1InputNative {
  public delegate bool EnumProc(IntPtr h, IntPtr state);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr s);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder t, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int index);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool RegisterHotKey(IntPtr h, int id, uint mods, uint vk);
  [DllImport("user32.dll")] public static extern bool UnregisterHotKey(IntPtr h, int id);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public struct Rect { public int Left,Top,Right,Bottom; }
}
'@
Add-Type -TypeDefinition $source

function Send-Hotkey([byte]$key) {
  [N1InputNative]::keybd_event(0x11,0,0,[UIntPtr]::Zero); [N1InputNative]::keybd_event(0x12,0,0,[UIntPtr]::Zero)
  [N1InputNative]::keybd_event($key,0,0,[UIntPtr]::Zero); [N1InputNative]::keybd_event($key,0,2,[UIntPtr]::Zero)
  [N1InputNative]::keybd_event(0x12,0,2,[UIntPtr]::Zero); [N1InputNative]::keybd_event(0x11,0,2,[UIntPtr]::Zero)
}
function Send-XButton1 {
  [N1InputNative]::mouse_event(0x0080,0,0,1,[UIntPtr]::Zero); [N1InputNative]::mouse_event(0x0100,0,0,1,[UIntPtr]::Zero)
}
function Get-Launcher([int]$processId) {
  $found=[Collections.Generic.List[object]]::new()
  [void][N1InputNative]::EnumWindows({param($h,$s) $owner=0;[void][N1InputNative]::GetWindowThreadProcessId($h,[ref]$owner);if($owner -eq $processId){$t=[Text.StringBuilder]::new(128);[void][N1InputNative]::GetWindowText($h,$t,128);$r=New-Object N1InputNative+Rect;[void][N1InputNative]::GetWindowRect($h,[ref]$r);$width=$r.Right-$r.Left;$area=$width*($r.Bottom-$r.Top);$topmost=([N1InputNative]::GetWindowLong($h,-20) -band 8) -ne 0;if($t.ToString() -eq 'N1_Recipe' -and $width -gt 500 -and $topmost){$found.Add([pscustomobject]@{Handle=$h;Area=$area})}};return $true},[IntPtr]::Zero)
  $found|Sort-Object Area|Select-Object -First 1
}
function Wait-Visible([IntPtr]$h,[bool]$expected) { for($i=0;$i -lt 100;$i++){if([N1InputNative]::IsWindowVisible($h) -eq $expected){return $true};Start-Sleep -Milliseconds 20};return $false }
function Invoke-Cycles([IntPtr]$h,[scriptblock]$trigger,[int]$count,[string]$name) {
  for($i=1;$i -le $count;$i++){if($name -ne 'mouse' -and $script:foregroundWindows.Count -gt 0){$fg=$script:foregroundWindows[($i-1)%$script:foregroundWindows.Count];[void][N1InputNative]::SetForegroundWindow($fg);Start-Sleep -Milliseconds 80};&$trigger;if(-not(Wait-Visible $h $true)){throw "$name cycle $i did not show Launcher"};Start-Sleep -Milliseconds 100;[void][N1InputNative]::SendMessage($h,0x0010,[IntPtr]::Zero,[IntPtr]::Zero);if(-not(Wait-Visible $h $false)){throw "$name cycle $i did not hide Launcher"};Start-Sleep -Milliseconds 180}
}

if(Get-Process n1_recipe -ErrorAction SilentlyContinue){throw 'Close existing n1_recipe processes first.'}
$held=$false
if($Mode -eq 'ConflictRecovery'){$held=[N1InputNative]::RegisterHotKey([IntPtr]::Zero,48879,3,0x7A);if(-not $held){throw 'Could not reserve Ctrl+Alt+F11 for conflict test.'}}
$app=Start-Process -FilePath $exe -PassThru
try {
  $launcher=$null;for($i=0;$i -lt 100 -and -not $launcher;$i++){Start-Sleep -Milliseconds 50;$launcher=Get-Launcher $app.Id};if(-not $launcher){throw 'Launcher window not found.'}
  # Window creation precedes tray/hotkey/hook setup; do not race registration.
  Start-Sleep -Milliseconds 1000
  $script:foregroundWindows=@(Get-Process Code,chrome,WindowsTerminal -ErrorAction SilentlyContinue | Where-Object MainWindowHandle -ne 0 | Select-Object -ExpandProperty MainWindowHandle)
  Write-Verbose ("Launcher handle={0} area={1} visible={2}" -f $launcher.Handle,$launcher.Area,[N1InputNative]::IsWindowVisible($launcher.Handle)) -Verbose
  if([N1InputNative]::IsWindowVisible($launcher.Handle)){[void][N1InputNative]::SendMessage($launcher.Handle,0x0010,[IntPtr]::Zero,[IntPtr]::Zero);[void](Wait-Visible $launcher.Handle $false)}
  if($Mode -eq 'Stress') {
    Invoke-Cycles $launcher.Handle {Send-Hotkey 0x7A} $Cycles 'primary'
    Invoke-Cycles $launcher.Handle {Send-Hotkey 0x7B} $Cycles 'secondary'
    $app.Refresh(); if($app.MainWindowHandle -ne 0){[void][N1InputNative]::SetForegroundWindow($app.MainWindowHandle);Start-Sleep -Milliseconds 100}
    Invoke-Cycles $launcher.Handle {Send-XButton1} $Cycles 'mouse'
    $second=Start-Process -FilePath $exe -PassThru; if(-not $second.WaitForExit(5000)){Stop-Process -Id $second.Id -Force;throw 'Second instance did not exit.'};if(-not(Get-Process -Id $app.Id -ErrorAction SilentlyContinue)){throw 'First instance exited.'}
  } elseif($Mode -eq 'MouseOnly') {
    $app.Refresh(); if($app.MainWindowHandle -ne 0){[void][N1InputNative]::SetForegroundWindow($app.MainWindowHandle);Start-Sleep -Milliseconds 100}
    Invoke-Cycles $launcher.Handle {Send-XButton1} $Cycles 'mouse'
  } else {
    Send-Hotkey 0x7B;if(-not(Wait-Visible $launcher.Handle $true)){throw 'Backup shortcut failed while primary conflicted.'};[void][N1InputNative]::SendMessage($launcher.Handle,0x0010,[IntPtr]::Zero,[IntPtr]::Zero);[void](Wait-Visible $launcher.Handle $false)
    [void][N1InputNative]::UnregisterHotKey([IntPtr]::Zero,48879);$held=$false;Start-Sleep -Seconds 3
    Send-Hotkey 0x7A;if(-not(Wait-Visible $launcher.Handle $true)){throw 'Primary did not recover after conflict release.'}
  }
  [pscustomobject]@{Result='PASS';Mode=$Mode;Cycles=$Cycles;ProcessId=$app.Id;Executable=$exe}
} finally {
  if($held){[void][N1InputNative]::UnregisterHotKey([IntPtr]::Zero,48879)}
  if(Get-Process -Id $app.Id -ErrorAction SilentlyContinue){
    Stop-Process -Id $app.Id -Force
    Wait-Process -Id $app.Id -ErrorAction SilentlyContinue
  }
}
