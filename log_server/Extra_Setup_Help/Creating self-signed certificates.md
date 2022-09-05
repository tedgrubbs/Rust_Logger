>*For testing purposes it is necessary to generate your own self-signed keys for local testing of the log_server. In production you should always get keys from a reputable certificate authority.*

The below command will create a new cert/key pair for a TLS server. You only need to run this on the machine that will host the server itself.

```bash
openssl req -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out myserver.crt -keyout myserver.key
```
You will then need to add this cert to your list of trusted certificates:

```bash
cp <crt file> /usr/share/ca-certificates/
dpkg-reconfigure ca-certificates # this shows you a list of all the certs on your machine. Will need to add the new one here.
```

__Code note__: For Rustls/hyper server need to make sure key file matches rustls_pemfile loading function. 
IE: BEGIN PRIVATE KEY means this is a PKCS8 key and should be loaded with rustls_pemfile::pkcs8_private_keys.
