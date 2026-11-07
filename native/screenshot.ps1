param([string]$Out = "D:\Github\InsynDoc\native\shot.png")
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
$b = New-Object System.Drawing.Bitmap($vs.Width, $vs.Height)
$g = [System.Drawing.Graphics]::FromImage($b)
$g.CopyFromScreen($vs.Left, $vs.Top, 0, 0, $b.Size)
$b.Save($Out)
$g.Dispose(); $b.Dispose()
Write-Output "saved $Out"
