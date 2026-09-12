# Authentication

Spotiline uses Spotify's OAuth 2.0 flow to authenticate with your account.

## First-Time Setup

1. **Get Spotify API Credentials:**
   - Visit [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Create an app
   - Note your Client ID and Client Secret
   - Add `http://127.0.0.1:8888/callback` as a redirect URI

2. **Start the daemon:**
   ```bash
   spotiline daemon start
   ```

3. **Follow the automatic authorization:**
   - Enter your Client ID and Client Secret when prompted
   - Your default browser opens automatically to Spotify's authorization page
   - Click "Agree" to authorize Spotiline
   - You see a success page: "Authorization Complete!"
   - Return to your terminal - setup is done!

4. **Credentials are stored securely:**
   - Tokens saved to OS keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)
   - No passwords stored in plain text

## Environment Variables (Optional)

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
export SPOTIFY_CLIENT_SECRET="your_client_secret"
```

If set, Spotiline will use these instead of prompting.

## Manual Authorization (Fallback)

If the automatic browser flow doesn't work (firewall, port conflict, headless environment):

1. When prompted, the terminal shows a URL
2. Manually copy and open it in a browser
3. After authorizing, your browser shows "This site can't be reached" - **this is expected**
4. Copy the **full URL** from your browser's address bar (including `http://127.0.0.1:8888/callback?code=...`)
5. Paste it back into the terminal

## Troubleshooting

### Auto-capture doesn't work
If browser doesn't auto-open or callback fails:
- **Firewall:** Ensure localhost (127.0.0.1) port 8888 is allowed
- **Port conflict:** Another app may be using port 8888
- **SSH/Remote:** Auto-capture requires a local browser - use manual mode
- **Solution:** Spotiline automatically falls back to manual authorization with clear instructions

### Browser opens but hangs
- Click "Agree" on the Spotify authorization page within 120 seconds
- If timeout occurs, restart: `spotiline daemon start`

### "This site can't be reached" error page
**During manual mode only:**
- This is expected behavior when using manual authorization
- Copy the full URL from the address bar and paste it into the terminal
- The URL contains your authorization code even though the page shows an error

**During auto-capture mode:**
- You should see a success page instead
- If you see an error page, auto-capture failed - follow the manual instructions shown in the terminal
