# Authentication

Spotiline uses Spotify's OAuth 2.0 flow to authenticate with your account.

## First-Time Setup

1. **Get Spotify API Credentials:**
   - Visit [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Create an app
   - Note your Client ID and Client Secret
   - Add `http://localhost:8888/callback` as a redirect URI

2. **Start the daemon:**
   ```bash
   spotiline daemon start
   ```

3. **Follow the prompts:**
   - Enter your Client ID and Client Secret
   - Open the authorization URL in your browser
   - After authorizing, paste the full redirect URL back

4. **Credentials are stored securely:**
   - Tokens saved to OS keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)
   - No passwords stored in plain text

## Environment Variables (Optional)

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
export SPOTIFY_CLIENT_SECRET="your_client_secret"
```

If set, Spotiline will use these instead of prompting.
