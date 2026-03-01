# Post-process an MSI to set per-user installation (no UAC prompt).
# Equivalent to editing with Orca: ALLUSERS=2, MSIINSTALLPERUSER=1.
param(
    [Parameter(Mandatory)][string]$MsiPath
)

$ErrorActionPreference = "Stop"

$MsiPath = Resolve-Path $MsiPath

$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase($MsiPath, 1) # 1 = msiOpenDatabaseModeTransact

function Set-MsiProperty($db, $name, $value) {
    $view = $db.OpenView("SELECT * FROM Property WHERE Property = '$name'")
    $view.Execute()
    $row = $view.Fetch()
    if ($row) {
        $row.StringData(2) = $value
        $view.Modify(2, $row) # 2 = msiViewModifyUpdate
    } else {
        $view.Close()
        $view = $db.OpenView("INSERT INTO Property (Property, Value) VALUES ('$name', '$value')")
        $view.Execute()
    }
    $view.Close()
}

Set-MsiProperty $database "ALLUSERS" "2"
Set-MsiProperty $database "MSIINSTALLPERUSER" "1"

$database.Commit()

Write-Host "Patched $MsiPath : ALLUSERS=2, MSIINSTALLPERUSER=1"
