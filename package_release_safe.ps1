$env:PATH = "C:\Program Files\Git\bin;C:\Program Files\GitHub CLI;" + $env:PATH

$Stage = "Volt_App_Stage";
$ZipFile = "Volt_App_v1.0.20.zip";

# Clean Start
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue;
Remove-Item $ZipFile -Force -ErrorAction SilentlyContinue;

New-Item -ItemType Directory -Force -Path $Stage;

# 1. Restore SAFE Binary (v1.0.18 - Localhost)
$RestoreDir = "Temp_Restore";
Remove-Item $RestoreDir -Recurse -Force -ErrorAction SilentlyContinue;
Expand-Archive -Path "Volt_App_v1.0.18.zip" -DestinationPath $RestoreDir -Force;

if (Test-Path "$RestoreDir/volt_wallet.exe") {
    Copy-Item "$RestoreDir/volt_wallet.exe" -Destination "$Stage/volt_wallet.exe" -Force;
} elseif (Test-Path "$RestoreDir/Volt_App_Final/volt_wallet.exe") {
    Copy-Item "$RestoreDir/Volt_App_Final/volt_wallet.exe" -Destination "$Stage/volt_wallet.exe" -Force;
} else {
    Write-Error "Could not find volt_wallet.exe in v1.0.18 zip restore.";
    exit 1;
}
Remove-Item $RestoreDir -Recurse -Force;

# 2. Copy other assets from Final
$Final = "Volt_App_Final";
$Exclude = @("volt_wallet.exe", "*.zip", "LOCK"); 
if (Test-Path $Final) {
    Get-ChildItem -Path $Final -Exclude $Exclude | Copy-Item -Destination $Stage -Recurse -Force;
} else {
    Write-Warning "Volt_App_Final not found. Release might be incomplete assets.";
}

# 3. Zip
Compress-Archive -Path "$Stage/*" -DestinationPath $ZipFile -Force;

# 4. Git Tag & Release
cd volt_core_github;
git fetch --tags;
git tag -f v1.0.20;
git push origin v1.0.20 -f;
cd ..;

gh release delete v1.0.20 -y --cleanup-tag;
gh release create v1.0.20 --title "v1.0.20 - Security Fix" --notes "CRITICAL SECURITY UPDATE: Reverted default connection to Localhost to prevent shared wallet access." --target main;
gh release upload v1.0.20 $ZipFile --clobber
