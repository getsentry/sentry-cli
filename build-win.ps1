# This script helps compiling the thing for windows.  Reason this is not as
# trivial is because we want to compile in the openssl library as a static
# library instead of shipping DLLs.
#
# This requires that we have a matching compiler.  Since compiling openssl
# on windows is a nightmare we download a precompiled version that is compiled
# with vs2015 which is also what we assume the rest is compiled with.

$openssl_version = "openssl-1.0.1t-vs2015"
$openssl_path = Join-Path (pwd) ("vendor/{0}" -f $openssl_version)
$openssl_url = "http://www.npcglib.org/~stathis/downloads/{0}.7z" -f $openssl_version;

if (!(Test-Path $openssl_path)) {
    Invoke-WebRequest -Uri $openssl_url -OutFile "vendor/openssl.7z"
    7z x -y vendor/openssl.7z -ovendor
}

$env:OPENSSL_LIBS = "ssleay32MT:libeay32MT"
$env:OPENSSL_LIB_DIR = Join-Path $openssl_path "lib"
$env:OPENSSL_INCLUDE_DIR = Join-Path $openssl_path "include"
$env:OPENSSL_STATIC = "1"

cargo build --release --target i686-pc-windows-msvc
