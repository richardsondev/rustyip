<p align="center">
  <img src="icon/rustyip.png" alt="RustyIP icon" width="128" />
</p>

# RustyIP client
A containerized version of a proprietary dynamic DNS client. Backend not included.

## Container support
* ARM32
* ARM64
* X64

## Source
https://github.com/richardsondev/rustyip

## Prebuilt image
https://hub.docker.com/r/richardsondev/rustyip

## Usage
Generate a key. Keep the private seed on the client (`KEY`); register the public key with the backend:
```bash
docker run --rm richardsondev/rustyip:latest keygen
```

Run the client:
```bash
docker run -d --name=watchtower -v /var/run/docker.sock:/var/run/docker.sock containrrr/watchtower
docker run -d --name=RustyIP --restart always --label=com.centurylinklabs.watchtower.enable=true -e HOST='' -e HASH='' -e KEY='' -e SLEEP_DURATION='5' richardsondev/rustyip:latest
```

| Variable | Required | Description |
| --- | --- | --- |
| `HOST` | yes | Backend host name. |
| `HASH` | yes | Account identifier. |
| `KEY` | yes | Base64 Ed25519 private seed (from `keygen`). |
| `KID` | no | Key id sent with each update (default `1`). |
| `SLEEP_DURATION` | no | Minutes between updates (default `5`). |

## License
MIT
