-- Add optional webhook validation fields to endpoints.
-- validation_type: 'none', 'hmac_sha256', 'hmac_sha512', 'header_token', 'query_token'
-- validation_secret: the shared secret or expected token value
-- validation_header: header name for signature or token (e.g. 'x-hub-signature-256')
-- validation_query: query parameter name for token (e.g. 'token')

ALTER TABLE endpoints ADD COLUMN validation_type TEXT NOT NULL DEFAULT 'none';
ALTER TABLE endpoints ADD COLUMN validation_secret TEXT;
ALTER TABLE endpoints ADD COLUMN validation_header TEXT;
ALTER TABLE endpoints ADD COLUMN validation_query TEXT;
