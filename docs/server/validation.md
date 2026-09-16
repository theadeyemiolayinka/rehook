# Webhook Validation

HookRelay can optionally validate that inbound webhooks are authentic before storing them. This prevents unauthorized requests from being captured as events.

## How it works

Each endpoint can be configured with a validation method. When a request arrives at the endpoint URL, the server validates it before storing the event. If validation fails, the server responds with 403 Forbidden and the request is not stored.

## Validation methods

### None

No validation. All requests to the endpoint URL are captured. This is the default.

### HMAC-SHA256

The server computes an HMAC-SHA256 of the request body using a shared secret and compares it to a signature header. This is the standard method used by Stripe, GitHub, and many other providers.

Configuration:
- **Secret**: the shared secret provided by the webhook provider
- **Header**: the header that carries the signature (e.g. `x-hub-signature-256` for GitHub, `stripe-signature` for Stripe)

The server accepts both raw hex digests and prefixed formats like `sha256=abc123...`.

### HMAC-SHA512

Same as HMAC-SHA256 but using SHA-512. Used by Paystack and some other providers.

Configuration:
- **Secret**: the shared secret
- **Header**: the signature header (e.g. `x-paystack-signature` for Paystack)

### Header token

The server checks that a specific header equals a configured value. This is a simple shared-secret approach.

Configuration:
- **Secret**: the expected token value
- **Header**: the header name (e.g. `x-api-key`)

### Query parameter token

The server checks that a query parameter in the URL equals a configured value. Useful for providers that support adding a token to the webhook URL.

Configuration:
- **Secret**: the expected token value
- **Query parameter**: the parameter name (e.g. `token`, so the webhook URL is `https://.../i/abc?token=secret`)

## Common providers

| Provider | Method | Header | Notes |
|----------|--------|--------|-------|
| Stripe | HMAC-SHA256 | `stripe-signature` | The secret starts with `whsec_` |
| GitHub | HMAC-SHA256 | `x-hub-signature-256` | Set in webhook settings |
| Paystack | HMAC-SHA512 | `x-paystack-signature` | Uses your secret key |
| Shopify | HMAC-SHA256 | `x-shopify-hmac-sha256` | Base64-encoded signature |
| Generic | Header token | `x-api-key` | Any shared secret header |

## Configuring validation

1. Go to your project in the admin dashboard.
2. Click the edit icon on an endpoint, or create a new endpoint.
3. Select the validation method.
4. Fill in the required fields (secret, header name, or query parameter name).
5. Save.

The endpoint list shows a badge for endpoints that have validation configured.

## Security notes

- The secret is stored in the server database. Use a strong, unique secret per endpoint.
- The secret is never sent to agents or included in delivery instructions.
- Validation happens before the event is stored. Failed requests are rejected with 403 and do not create events.
- If your provider does not support any of the listed methods, you can use the query parameter token approach by appending a secret to the webhook URL (e.g. `https://.../i/abc?token=your-secret`).
