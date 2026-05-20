# Birthday Reminders MCP Configuration Guide

This guide helps MCP clients (LM Studio, Hermes, Claude Desktop, Cursor, and others) connect to and use the Birthday Reminders API endpoint.

## Endpoint Details

- **URL**: `http://localhost:3000/mcp` (local development) or `https://your-host.example.com/mcp` (production)
- **Transport**: Streamable HTTP (rmcp) or HTTP
- **Authentication**: API token via `Authorization: Bearer <token>` header — authenticated once per MCP session
- **Tools Available**:
  - `list_birthdays` — List all birthdays
  - `upcoming_birthdays` — List upcoming birthdays (configurable days ahead)
  - `get_birthday_by_name` — Look up a person's birthday by name (case-insensitive partial match)
  - `add_birthday` — Add a new birthday with optional contact fields
  - `remove_birthday` — Not supported; use web interface instead

## Setup Instructions

### Step 1: Generate an API Token

1. Open the Birthday Reminders web UI
2. Navigate to **Settings → API Tokens**
3. Click **Generate New Token**
4. Enter a token name (e.g., "LM Studio", "Hermes Local")
5. **Copy the token immediately** — it will not be shown again
6. Store it securely in your environment

### Step 2: Store Token in Environment

**Linux/macOS:**
```bash
export BIRTHDAY_API_TOKEN="your-token-here"
```

**Windows (PowerShell):**
```powershell
$env:BIRTHDAY_API_TOKEN="your-token-here"
```

**Permanently (macOS/Linux):**
Add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):
```bash
export BIRTHDAY_API_TOKEN="your-token-here"
```

### Step 3: Configure Your MCP Client

#### LM Studio

1. Go to **Servers** → **MCP Servers**
2. Add a new server:
   - **Name**: `birthday-reminders`
   - **URL**: `http://localhost:3000/mcp`
   - **Type**: `Streamable HTTP`
3. Add the environment variable:
   - **Key**: `BIRTHDAY_API_TOKEN`
   - **Value**: (leave empty — client will read from system environment)
4. If custom headers are supported in your LM Studio version, set:
  - `Authorization: Bearer ${BIRTHDAY_API_TOKEN}`

#### Hermes

1. Edit your Hermes config file (usually `~/.hermes/config.yaml` or similar)
2. Add to the MCP servers section:
   ```yaml
   mcpServers:
     birthday-reminders:
       transport: streamable_http
       url: http://localhost:3000/mcp
       headers:
         Authorization: "Bearer ${BIRTHDAY_API_TOKEN}"
   ```
3. Set `BIRTHDAY_API_TOKEN` in your system environment before launching Hermes

#### Claude Desktop

1. Edit the Claude Desktop config:
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
2. Add to `mcpServers`:
   ```json
   {
     "mcpServers": {
       "birthday-reminders": {
         "transport": "streamable_http",
          "url": "http://localhost:3000/mcp",
          "headers": {
            "Authorization": "Bearer ${BIRTHDAY_API_TOKEN}"
          }
       }
     }
   }
   ```
3. Set `BIRTHDAY_API_TOKEN` in your system environment

#### Cursor

1. Open Cursor settings
2. Find the **MCP Servers** section
3. Add:
   - **Name**: `birthday-reminders`
   - **URL**: `http://localhost:3000/mcp`
   - **Transport**: `streamable_http` or `http`
4. Set header (recommended when available):
  - `Authorization: Bearer ${BIRTHDAY_API_TOKEN}`
5. Set `BIRTHDAY_API_TOKEN` in your system environment

#### Generic MCP Client

If your client supports `mcpServers` JSON:
```json
{
  "mcpServers": {
    "birthday-reminders": {
      "transport": "streamable_http",
      "url": "http://localhost:3000/mcp",
      "headers": {
        "Authorization": "Bearer ${BIRTHDAY_API_TOKEN}"
      }
    }
  }
}
```

## HTTP Header Authentication

Configure your MCP client with a global `Authorization` header to authenticate the MCP session:

```http
Authorization: Bearer $BIRTHDAY_API_TOKEN
```

The token is validated once when the MCP session is initialized. All subsequent tool calls within the same session are automatically authenticated — no per-tool token is needed.

## Tool Usage Examples

### list_birthdays

Returns all birthdays for the authenticated user.

```json
{}
```

Response:
```json
{
  "birthdays": [
    {
      "id": "uuid-here",
      "name": "Jane Doe",
      "birth_date": "1990-05-15",
      "age": 35,
      "turning_age": 36,
      "days_until": 42,
      "email": "jane@example.com",
      "phone_number": "+31 6 12345678",
      "address": "Keizersgracht 1",
      "postal_code": "1015 CC",
      "city": "Amsterdam",
      "country": "Netherlands",
      "notes": "Likes coffee"
    }
  ]
}
```

### upcoming_birthdays

List birthdays happening in the next N days.

```json
{
  "days": 30
}
```

### get_birthday_by_name

Look up birthdays by name. Uses case-insensitive substring matching, so partial names work (e.g. `"Anna"` matches `"Anna Smith"` and `"Anna Jones"`).

```json
{
  "name": "Jane"
}
```

Response:
```json
{
  "count": 1,
  "matches": [
    {
      "id": "uuid-here",
      "name": "Jane Doe",
      "birth_date": "1990-05-15",
      "age": 35,
      "turning_age": 36,
      "days_until": 0,
      "email": "jane@example.com",
      "phone_number": null,
      "address": null,
      "postal_code": null,
      "city": null,
      "country": null,
      "notes": null
    }
  ]
}
```

If no match is found, `count` is `0` and `matches` is an empty array.

### add_birthday

Add a new birthday.

```json
{
  "name": "John Smith",
  "birth_date": "1985-12-25",
  "email": "john@example.com",
  "phone_number": "+1-555-1234",
  "address": "123 Main St",
  "postal_code": "90210",
  "city": "Beverly Hills",
  "country": "USA",
  "notes": "Likes chocolate"
}
```

If you use header authentication, all fields except `name` and `birth_date` are optional.

### remove_birthday

Not supported via MCP. Use the web interface to delete birthdays.

## Session-Based Authentication

**How it works**: Your API token is verified once when the MCP session is initialized. The server binds your identity to the MCP session, so subsequent tool calls are authenticated automatically.

### Setup:

1. **Environment Variables** (Recommended)
   - Set `BIRTHDAY_API_TOKEN` in your system environment
   - Most secure for local clients

2. **MCP Config Header**
   - Configure your MCP server entry with:
   - `Authorization: Bearer ${BIRTHDAY_API_TOKEN}`
   - Best for clients that support static/dynamic headers per MCP server

## Security Best Practices

- **Never** commit API tokens to version control
- **Never** share tokens publicly
- Use environment variables for local/desktop clients (LM Studio, Hermes, Cursor)
- Create a dedicated token for each client/device
- Revoke tokens you no longer use in **Settings → API Tokens**
- For shared/cloud clients, use tokens with limited lifetime if available
- Treat API tokens like passwords

## Troubleshooting

### "Invalid API token" Error

- Verify the token is set correctly in your environment: `echo $BIRTHDAY_API_TOKEN`
- Check the token hasn't been revoked in the web UI
- Create a new token if unsure

### Tool Not Found

- Ensure the MCP endpoint is correctly configured
- Verify the server URL is reachable: `curl http://localhost:3000/mcp`
- Check that `mcp.enabled: true` in `config.yaml`

### Token Lost After Restart

- MCP is stateless; you must provide the token with every tool call
- Use environment variables to avoid re-typing
- Or add the token to your client's system prompt

### Connection Refused

- Ensure Birthday Reminders server is running
- Check the endpoint URL matches your deployment (localhost vs. domain)
- Verify firewall/network access to the endpoint

## Advanced: Remote Deployments

If running Birthday Reminders remotely:

1. Update the URL in your MCP client config:
   ```json
   "url": "https://birthdays.example.com/mcp"
   ```

2. Ensure `server.server_name` in `config.yaml` matches:
   ```yaml
   server:
     server_name: "birthdays.example.com"
     scheme: "https"
   ```

3. Store the remote token in `BIRTHDAY_API_TOKEN` (same as local)

## Integration Examples

### Ask a Language Model to List Birthdays

Prompt:
```
Call the list_birthdays tool to show me all stored birthdays.
Prefer MCP header authentication with Authorization: Bearer $BIRTHDAY_API_TOKEN.
If headers are unavailable, include token from BIRTHDAY_API_TOKEN in tool parameters.
```

### Schedule a Reminder Check

Prompt:
```
Use the upcoming_birthdays tool to find birthdays in the next 7 days.
Prefer MCP header authentication with Authorization: Bearer $BIRTHDAY_API_TOKEN.
If headers are unavailable, include token: $BIRTHDAY_API_TOKEN
```

### Look Up Someone's Birthday

Use the `get_birthday_by_name` tool whenever the user asks questions such as:

- "What is Anna's birthday?"
- "When is the birthday of Anna?"
- "Can you tell me when Anna is aging?"
- "Can you tell me the birthday of Anna?"
- "How old is Anna turning?"
- "When does Anna celebrate her birthday?"

For any of these patterns, extract the person's name and call `get_birthday_by_name`. The tool uses case-insensitive substring matching, so a partial name like `"Anna"` will match `"Anna Smith"` and `"Anna Jones"`.

Prompt:
```
When is Anna's birthday? Use the get_birthday_by_name tool with name "Anna".
Prefer MCP header authentication with Authorization: Bearer $BIRTHDAY_API_TOKEN.
If headers are unavailable, include token: $BIRTHDAY_API_TOKEN
```

The tool returns all matches with their birth date, current age, days until next birthday, and contact details.

### Add a Birthday from Conversation

Prompt:
```
Add a birthday for Sarah (born 1992-03-15) using the add_birthday tool.
Prefer MCP header authentication with Authorization: Bearer $BIRTHDAY_API_TOKEN.
If headers are unavailable, include token: $BIRTHDAY_API_TOKEN
Contact: sarah@example.com
Phone: +1-555-9876
City: Portland, OR
```

## Support & Documentation

- **Full Documentation**: See [README.md](README.md) for complete setup instructions
- **API Token Management**: [Settings → API Tokens](http://localhost:3000/settings/api-tokens)
- **Web UI**: [Dashboard](http://localhost:3000/)
