# rhttpfile

Rust based web application to upload and download files. Simple as possible, only one executable is needed to run. 
Can be used in situations where a simple solution for file exchange is needed. 
Encryption enabled by default (self signed). Certificate can be exported from /certs folder.

## Installation

```bash
cargo run
```

Download prebuilt package from releases.
```bash
./rhttpfile
```


## Usage

```bash
./rhttpfile --help
Simple http server to upload and download files

Usage: rhttpfile [OPTIONS] [ADDR]

Arguments:
  [ADDR]  ip_addr e.g. 127.0.0.1 [default: 127.0.0.1]

Options:
  -p, --port <PORT>  ip_addr e.g. 8080 [default: 8080]
  -h, --help         Print help
  -V, --version      Print version

```

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

[MIT](https://choosealicense.com/licenses/mit/)

