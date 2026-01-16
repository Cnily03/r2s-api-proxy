# r2s-api-proxy

Reverse proxy for Ret2Shell API with fixed token.

## Features

You can use fixed token in Authorization header like `Bearer custom_key`, and the proxy server will transfer to `Bearer eyJ...`. The Ret2Shell token `eyJ...` will be automatically refreshed.

## Usage

```bash
Usage: r2s-api-proxy [OPTIONS] --endpoint <ENDPOINT>

Options:
      --endpoint <ENDPOINT>            The endpoint to proxy requests to
      --key <KEY>                      Authorization keys (can be specified multiple times)
  -i, --ping-interval <PING_INTERVAL>  Ping interval in seconds [default: 1800]
  -H, --host <HOST>                    Host to listen on [default: 0.0.0.0]
  -p, --port <PORT>                    Port to listen on [default: 8080]
      --base <BASE>                    Base path for the proxy [default: /]
  -h, --help                           Print help
```

## License

Copyright (c) Cnily03.

Licensed under the [MIT License](LICENSE).
