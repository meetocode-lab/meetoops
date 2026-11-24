# MeetoOps — First-Run Setup Guide

When MeetoOps starts against a fresh installation, it automatically enters the **First-Run Setup Wizard**. No terminal commands or configuration files are required.

---

## 1. Initialization State Machine

```text
UNINITIALIZED -> FIRST_RUN_SETUP -> ADMIN_CREATED -> INFRASTRUCTURE_SETUP -> READY
```

- Persisted in the `app_state` singleton table.
- Protected by atomic database transactions to prevent concurrent admin creation races.
- Once completed (`READY`), all setup endpoints are permanently disabled (`403 Forbidden`) and unauthenticated visitors are redirected to `/login`.

---

## 2. Step-by-Step Walkthrough

### Step 1: Welcome
- Displays the MeetoOps Warm Obsidian welcome screen, tagline (*"Your infrastructure. One control center."*), system health/readiness verification, and the 5-step progress indicator.
- Click **Get Started**.

### Step 2: Create Administrator Account
- **Fields**: Full Name, Email Address, Username (optional), Password, Confirm Password.
- Live password strength validation enforces strong credentials (minimum 12 characters).
- Creates the initial **Super Administrator** (`super_admin`) using Argon2id hashing and automatically authenticates your browser session for the remaining setup steps.

### Step 3: Application Preferences *(Optional — Defaults Provided)*
- Configure Application Display Name, Timezone, Date Format, Time Format, and Default Monitoring Refresh Interval.
- You may click **Skip & Keep Defaults** to proceed immediately with sensible defaults.

### Step 4: Connect Docker Environment *(Optional — Can Configure Later)*
Choose how to connect your first server:
- **Option A — Local Docker Engine**:
  - MeetoOps automatically probes local Docker Engine availability.
  - Displays detected Docker version, Host OS, container count, and image count.
  - If the socket is not mounted or inaccessible, MeetoOps explains why clearly and provides safe guidance.
- **Option B — Remote Docker Server (MeetoOps Agent)**:
  - Enter a server name and generate a single-use, 15-minute registration token.
  - Displays the ready-to-run agent command and live connection status indicator.
- You may skip this step and connect servers later under **Servers**.

### Step 5: Installation Complete
- Displays a verified summary: Administrator account created, Database ready, Encryption vault active, and Docker/Agent connection status.
- Click **Go to Dashboard** to permanently lock the setup wizard (`READY`) and enter the MeetoOps Control Center.

---

## 3. Troubleshooting

- **Setup page no longer accessible after refresh**: If Step 2 completed, sign in at `/login` with your newly created Super Admin credentials to resume remaining setup steps.
- **Forgot initial administrator password**: First-run setup is never reopened automatically. Use the host CLI recovery command documented in [docs/SECURITY.md](./SECURITY.md#7-first-run-setup-security--recovery).
