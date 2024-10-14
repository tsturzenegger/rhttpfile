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

Usage: rhttpfile [OPTIONS] [ADDR] [PORT]

Arguments:
  [ADDR]  ip_addr e.g. 127.0.0.1 [default: 127.0.0.1]
  [PORT]  port e.g. 8080 [default: 8080]

Options:
  -u, --upload-limit <UPLOAD_LIMIT>
          upload limit (mebibytes) e.g. 1000 [default: 1000]
      --certs-dir <CERTS_DIR>
          directory with the tls certificates e.g. certs [default: certs]
      --key-file-name <KEY_FILE_NAME>
          file name of key e.g. key.pem [default: key.pem]
      --cert-file-name <CERT_FILE_NAME>
          file name of cert e.g. cert.pem [default: cert.pem]
      --subject-alt-name <SUBJECT_ALT_NAME>
          self signed cert subject alt name e.g. localhost [default: localhost]
  -h, --help
          Print help
  -V, --version
          Print version

```

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

[MIT](https://choosealicense.com/licenses/mit/)

