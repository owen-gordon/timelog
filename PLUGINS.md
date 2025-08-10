# Timelog Plugin System

The timelog tool supports a plugin system for uploading time entries to external platforms like Jira, Azure DevOps, Slack, or any HTTP API.

## Quick Start

1. **List available plugins:**
   ```bash
   timelog upload --list-plugins
   ```

2. **Upload today's entries using a specific plugin:**
   ```bash
   timelog upload --plugin demo today
   ```

3. **Dry run (preview without uploading):**
   ```bash
   timelog upload --plugin webhook today --dry-run
   ```

## Plugin Directory

Plugins are stored in `~/.timelog/plugins/` (or `$TIMELOG_PLUGIN_PATH`)

- Plugin executables: `timelog-<name>` (must be executable)
- Plugin configs: `timelog-<name>.json` (optional)

## Included Plugins

### Demo Plugin (`demo`)
A simple example plugin that shows how the interface works.

### Webhook Plugin (`webhook`)
Posts time entries to any HTTP endpoint.

**Config** (`~/.timelog/plugins/timelog-webhook.json`):
```json
{
  "webhook_url": "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK",
  "webhook_token": "optional-bearer-token",
  "format": "batch"
}
```

- `format`: "batch" (all records in one request) or "individual" (one per record)

### Jira Plugin (`jira`)
Uploads time entries as work logs to Jira issues.

**Config** (`~/.timelog/plugins/timelog-jira.json`):
```json
{
  "jira_url": "https://your-org.atlassian.net",
  "jira_email": "your-email@company.com", 
  "jira_token": "your-api-token-here",
  "default_issue": "PROJ-123",
  "task_issue_map": {
    "coding": "DEV-456",
    "testing": "QA-789"
  }
}
```

**Issue Selection Priority:**
1. `task_issue_map` - exact task name match
2. Issue key in task name (e.g., "PROJ-123: Fix bug") 
3. `default_issue` fallback

**Get Jira API Token:** https://id.atlassian.com/manage-profile/security/api-tokens

### Azure DevOps Plugin (`ado`)
Uploads time entries as completed work to Azure DevOps work items.

**Config** (`~/.timelog/plugins/timelog-ado.json`):
```json
{
  "ado_organization": "your-org",
  "ado_project": "Your Project Name", 
  "ado_token": "your-personal-access-token-here",
  "default_work_item": "12345",
  "activity_type": "Development",
  "task_work_item_map": {
    "coding": "12345",
    "testing": "12346"
  }
}
```

**Work Item Selection Priority:**
1. `task_work_item_map` - exact task name match
2. Work item ID in task name (e.g., "1234: Fix bug" or "#1234 Fix bug")
3. `default_work_item` fallback

**Get ADO Personal Access Token:** https://dev.azure.com/[org]/_usersSettings/tokens
- Required scope: Work Items (read & write)

## Plugin Interface

Plugins receive JSON via stdin and output JSON to stdout.

**Input Format:**
```json
{
  "records": [
    {
      "task": "coding", 
      "duration_ms": 3600000,
      "date": "2025-08-10"
    }
  ],
  "period": "today",
  "config": {
    // contents of timelog-<plugin>.json
  }
}
```

**Output Format:**
```json
{
  "success": true,
  "uploaded_count": 1,
  "message": "Successfully processed 1 records",
  "errors": []
}
```

**Arguments:**
- `--dry-run`: Plugin should preview without uploading

## Writing Custom Plugins

1. Create executable script: `~/.timelog/plugins/timelog-<name>`
2. Optional config file: `~/.timelog/plugins/timelog-<name>.json`
3. Handle `--dry-run` argument
4. Read JSON from stdin, output JSON to stdout
5. Use stderr for progress messages

**Example plugin:**
```bash
#!/bin/bash
set -e

DRY_RUN=false
[[ "$1" == "--dry-run" ]] && DRY_RUN=true

INPUT=$(cat)
RECORDS=$(echo "$INPUT" | jq '.records')

# Process records...

jq -n '{success: true, uploaded_count: 1, message: "Done", errors: []}'
```

## Usage Examples

```bash
# Upload today's entries 
timelog upload demo today

# Upload this week with specific plugin
timelog upload --plugin jira this-week

# Upload to Azure DevOps
timelog upload --plugin ado this-month

# Preview last month without uploading
timelog upload --plugin webhook last-month --dry-run

# Auto-select plugin if only one available
timelog upload yesterday
```

## Troubleshooting

- **Plugin not found:** Ensure file is executable (`chmod +x`) and named `timelog-<name>`
- **Config errors:** Check JSON syntax in `timelog-<name>.json`
- **Multiple plugins:** Use `--plugin <name>` to specify which one
- **No plugins:** Use `--list-plugins` to see setup instructions
