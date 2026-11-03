$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path 'traces' | Out-Null
cargo test
cargo run --bin micro-os-demo | Tee-Object -FilePath 'traces/demo.log'
