# WIND (WIND Is Not DIM) POC

Workspace layout:
- crates/rim-common
- crates/rim-registry
- crates/rim-pub
- crates/rim-sub
- crates/rim-bench

Build:
```
  cargo build
```

Run:
```
  cargo run -p rim-registry -- 127.0.0.1:4500
  cargo run -p rim-pub -- --service SENSOR/TEMP --registry 127.0.0.1:4500 --payload-bytes 256 --hz 200
  cargo run -p rim-sub -- --mode monitored --bench-secs 0
```

Benchmark:
```
  cargo run -p rim-bench -- --subs 8 --payload-bytes 256 --hz 200 --secs 5
```
