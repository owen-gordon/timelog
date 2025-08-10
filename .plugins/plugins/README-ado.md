# Azure DevOps (ADO) Plugin for Timelog

This plugin uploads time entries from timelog to Azure DevOps work items as completed work hours.

## Setup

1. **Get a Personal Access Token:**
   - Go to https://dev.azure.com/[your-org]/_usersSettings/tokens
   - Create new token with "Work Items (read & write)" scope
   - Copy the token value

2. **Configure the plugin:**
   - Edit `timelog-ado.json` with your ADO details:
   ```json
   {
     "ado_organization": "your-org-name",
     "ado_project": "Your Project Name",
     "ado_token": "your-pat-token-here",
     "default_work_item": "12345"
   }
   ```

3. **Test the connection:**
   ```bash
   timelog upload --plugin ado today --dry-run
   ```

## How It Works

The plugin logs time entries to ADO work items by:
1. Converting duration from milliseconds to hours
2. Finding the target work item using:
   - Exact match in `task_work_item_map` 
   - Work item ID in task name (e.g., "1234: Fix login bug")
   - `default_work_item` as fallback
3. Adding completed work hours to the work item
4. Adding a history comment with the task description

## Work Item ID Detection

The plugin detects work item IDs from task names in these formats:
- `1234: Task description` 
- `#1234 Task description`
- `WI-1234: Task description` (extracts 1234)

## Configuration Options

- `ado_organization`: Your ADO organization name (from URL)
- `ado_project`: Project name within the organization  
- `ado_token`: Personal Access Token with Work Items permissions
- `default_work_item`: Fallback work item ID when none detected
- `activity_type`: Type of work being logged (default: "Development")
- `task_work_item_map`: Map specific task names to work item IDs

## Examples

```bash
# Upload today's work to ADO
timelog upload --plugin ado today

# Preview what would be uploaded
timelog upload --plugin ado this-week --dry-run

# Upload with specific work item mapping
# If task is "coding", it maps to work item in task_work_item_map
timelog start "coding"
# ... work for a while ...
timelog stop
timelog upload --plugin ado today
```

## Troubleshooting

- **403 Forbidden**: Check your PAT has Work Items (read & write) permissions
- **Work item not found**: Verify the work item ID exists and you have access
- **Invalid work item ID**: Ensure work item IDs are numeric
- **Missing organization/project**: Check the ADO URL format matches your config
