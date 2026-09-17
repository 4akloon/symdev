# Docker (Ubuntu 24.04 fallback)

Bind-mount a legally obtained S60 3rd Edition Feature Pack 2 SDK. Do not COPY SDK or ROM into the image. No EKA2L1.

```bash
docker build -t symdev-m0 -f docker/Dockerfile docker/
docker run --rm -it \
  -v /path/to/your/sdk:/opt/symbian/sdk \
  -e EPOCROOT=/opt/symbian/sdk \
  symdev-m0
```

`EPOCROOT` trailing `\` vs `/` on Linux: **Needs experiment**. ROM and EKA2L1 stay on the host.
