$env:Path = $env:Path + ';' + $env:USERPROFILE + '\.cargo\bin'
Set-Location D:\xi-system
rustc --version
cargo --version
