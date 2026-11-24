export type Language = 'en' | 'id';

export interface Translations {
  tagline: string;
  loading: string;
  retry: string;
  save: string;
  cancel: string;
  delete: string;
  confirm: string;
  unavailable: string;
  comingPhase: string;
  nav: {
    overview: string;
    dashboard: string;
    monitoring: string;
    alerts: string;
    docker: string;
    containers: string;
    images: string;
    volumes: string;
    networks: string;
    stacks: string;
    dataServices: string;
    discovery: string;
    databases: string;
    redis: string;
    operations: string;
    logs: string;
    terminal: string;
    servers: string;
    system: string;
    settings: string;
    auditLogs: string;
    signOut: string;
  };
  setup: {
    title: string;
    subtitle: string;
    stepWelcome: string;
    stepAdmin: string;
    stepPrefs: string;
    stepDocker: string;
    stepComplete: string;
    getStarted: string;
    createAdminTitle: string;
    createAdminDesc: string;
    fullName: string;
    email: string;
    usernameOpt: string;
    password: string;
    confirmPassword: string;
    passwordHint: string;
    createAdminBtn: string;
    prefsTitle: string;
    prefsDesc: string;
    skipDefaults: string;
    saveContinue: string;
    dockerTitle: string;
    dockerDesc: string;
    localDocker: string;
    remoteAgent: string;
    testLocal: string;
    connectLocal: string;
    generateToken: string;
    skipForNow: string;
    completeTitle: string;
    completeDesc: string;
    goToDashboard: string;
  };
  login: {
    title: string;
    subtitle: string;
    identifier: string;
    password: string;
    signIn: string;
  };
  dashboard: {
    title: string;
    subtitle: string;
    refresh: string;
    serversTotal: string;
    containersTotal: string;
    hostCpu: string;
    hostMemory: string;
    hostDisk: string;
    hostUptime: string;
    dockerEngine: string;
    recentAudit: string;
    noEvents: string;
  };
  settings: {
    title: string;
    subtitle: string;
    tabGeneral: string;
    tabUsers: string;
    tabSecurity: string;
    tabAudit: string;
    appName: string;
    appUrl: string;
    language: string;
    timezone: string;
    dateFormat: string;
    timeFormat: string;
    monitoringInterval: string;
    networkPortTitle: string;
    networkPortDesc: string;
    listenPort: string;
    activePort: string;
    checkPortBtn: string;
    restartNotice: string;
    saveSettings: string;
    savedToast: string;
    vaultStatus: string;
    vaultFingerprint: string;
    addUser: string;
  };
}

export const translations: Record<Language, Translations> = {
  en: {
    tagline: 'Your Infrastructure. One Control Center.',
    loading: 'Loading…',
    retry: 'Retry',
    save: 'Save',
    cancel: 'Cancel',
    delete: 'Delete',
    confirm: 'Confirm',
    unavailable: 'Unavailable',
    comingPhase: 'Scheduled for next phase',
    nav: {
      overview: 'Overview',
      dashboard: 'Dashboard',
      monitoring: 'Monitoring',
      alerts: 'Alerts',
      docker: 'Docker',
      containers: 'Containers',
      images: 'Images',
      volumes: 'Volumes',
      networks: 'Networks',
      stacks: 'Compose Stacks',
      dataServices: 'Data & Services',
      discovery: 'Service Discovery',
      databases: 'Database Explorer',
      redis: 'Redis Explorer',
      operations: 'Operations',
      logs: 'Log Center',
      terminal: 'Terminal',
      servers: 'Servers',
      system: 'System',
      settings: 'Settings',
      auditLogs: 'Audit Logs',
      signOut: 'Sign Out',
    },
    setup: {
      title: 'Welcome to MeetoOps',
      subtitle: 'Zero-configuration infrastructure control center. Complete the guided setup to initialize your workspace.',
      stepWelcome: 'Welcome',
      stepAdmin: 'Administrator',
      stepPrefs: 'Preferences',
      stepDocker: 'Docker',
      stepComplete: 'Complete',
      getStarted: 'Get Started',
      createAdminTitle: 'Create Super Administrator',
      createAdminDesc: 'This account holds full control over MeetoOps. Credentials are hashed with Argon2id and never stored in plaintext.',
      fullName: 'Full Name',
      email: 'Email Address',
      usernameOpt: 'Username (optional)',
      password: 'Password',
      confirmPassword: 'Confirm Password',
      passwordHint: 'Minimum 12 characters with uppercase, lowercase, and numbers.',
      createAdminBtn: 'Create Administrator & Continue',
      prefsTitle: 'Application Preferences',
      prefsDesc: 'Customize display and monitoring defaults. You can update these anytime in Settings.',
      skipDefaults: 'Skip & Keep Defaults',
      saveContinue: 'Save & Continue',
      dockerTitle: 'Connect First Docker Environment',
      dockerDesc: 'Probe your local Docker Engine or generate a secure outbound TLS registration token for a remote host.',
      localDocker: 'Local Docker Engine',
      remoteAgent: 'Remote Server (Agent)',
      testLocal: 'Test Local Connection',
      connectLocal: 'Connect Local Docker',
      generateToken: 'Generate Agent Token',
      skipForNow: 'Skip for Now',
      completeTitle: 'MeetoOps is Ready',
      completeDesc: 'Your administrator account, SQLite WAL database, and AEAD encryption vault are initialized.',
      goToDashboard: 'Go to Dashboard',
    },
    login: {
      title: 'Sign in to MeetoOps',
      subtitle: 'Enter your administrator or team credentials to access the control center.',
      identifier: 'Email or Username',
      password: 'Password',
      signIn: 'Sign In',
    },
    dashboard: {
      title: 'Infrastructure Dashboard',
      subtitle: 'Live host telemetry, Docker availability, and recent security events.',
      refresh: 'Refresh',
      serversTotal: 'Servers',
      containersTotal: 'Containers',
      hostCpu: 'Host CPU',
      hostMemory: 'Host Memory',
      hostDisk: 'Host Disk',
      hostUptime: 'System Uptime',
      dockerEngine: 'Local Docker Engine',
      recentAudit: 'Recent Audit Events',
      noEvents: 'No audit events recorded yet.',
    },
    settings: {
      title: 'Application Settings',
      subtitle: 'Manage runtime preferences, network listen port, users, and security policies without editing .env files.',
      tabGeneral: 'General & Network',
      tabUsers: 'Users & Roles',
      tabSecurity: 'Security & Vault',
      tabAudit: 'Audit Logs',
      appName: 'Application Name',
      appUrl: 'Public Application URL',
      language: 'Interface Language',
      timezone: 'Timezone',
      dateFormat: 'Date Format',
      timeFormat: 'Time Format',
      monitoringInterval: 'Monitoring Interval (seconds)',
      networkPortTitle: 'Web & API Listen Port',
      networkPortDesc: 'Check real-time OS port availability before switching ports to prevent collisions.',
      listenPort: 'Configured Listen Port',
      activePort: 'Currently Active Port',
      checkPortBtn: 'Check Availability',
      restartNotice: 'Port updated in database. Restart MeetoOps server/container (and update Docker port mapping if containerized) to bind the new port.',
      saveSettings: 'Save Settings',
      savedToast: 'Settings saved successfully.',
      vaultStatus: 'AES-256-GCM Secret Vault Active',
      vaultFingerprint: 'Master Key Fingerprint',
      addUser: 'Add User',
    },
  },
  id: {
    tagline: 'Infrastruktur Anda. Satu Pusat Kendali.',
    loading: 'Memuat…',
    retry: 'Coba Lagi',
    save: 'Simpan',
    cancel: 'Batal',
    delete: 'Hapus',
    confirm: 'Konfirmasi',
    unavailable: 'Tidak Tersedia',
    comingPhase: 'Dijadwalkan pada fase berikutnya',
    nav: {
      overview: 'Ringkasan',
      dashboard: 'Dasbor',
      monitoring: 'Pemantauan',
      alerts: 'Peringatan',
      docker: 'Docker',
      containers: 'Kontainer',
      images: 'Image',
      volumes: 'Volume',
      networks: 'Jaringan',
      stacks: 'Compose Stacks',
      dataServices: 'Data & Layanan',
      discovery: 'Deteksi Layanan',
      databases: 'Penjelajah Database',
      redis: 'Penjelajah Redis',
      operations: 'Operasional',
      logs: 'Pusat Log',
      terminal: 'Terminal',
      servers: 'Server',
      system: 'Sistem',
      settings: 'Pengaturan',
      auditLogs: 'Log Audit',
      signOut: 'Keluar',
    },
    setup: {
      title: 'Selamat Datang di MeetoOps',
      subtitle: 'Pusat kendali infrastruktur tanpa konfigurasi manual. Selesaikan panduan awal untuk menginisialisasi sistem Anda.',
      stepWelcome: 'Mulai',
      stepAdmin: 'Administrator',
      stepPrefs: 'Preferensi',
      stepDocker: 'Docker',
      stepComplete: 'Selesai',
      getStarted: 'Mulai Setup',
      createAdminTitle: 'Buat Super Administrator',
      createAdminDesc: 'Akun ini memiliki kendali penuh atas MeetoOps. Kata sandi di-hash dengan Argon2id dan tidak pernah disimpan dalam bentuk teks biasa.',
      fullName: 'Nama Lengkap',
      email: 'Alamat Email',
      usernameOpt: 'Username (opsional)',
      password: 'Kata Sandi',
      confirmPassword: 'Konfirmasi Kata Sandi',
      passwordHint: 'Minimal 12 karakter dengan huruf besar, huruf kecil, dan angka.',
      createAdminBtn: 'Buat Administrator & Lanjutkan',
      prefsTitle: 'Preferensi Aplikasi',
      prefsDesc: 'Sesuaikan tampilan, bahasa, dan interval pemantauan. Anda dapat mengubahnya kapan saja di Pengaturan.',
      skipDefaults: 'Lewati & Pakai Default',
      saveContinue: 'Simpan & Lanjutkan',
      dockerTitle: 'Hubungkan Lingkungan Docker Pertama',
      dockerDesc: 'Uji koneksi Docker Engine lokal atau buat token registrasi TLS aman untuk server jarak jauh.',
      localDocker: 'Docker Engine Lokal',
      remoteAgent: 'Server Jarak Jauh (Agent)',
      testLocal: 'Uji Koneksi Lokal',
      connectLocal: 'Hubungkan Docker Lokal',
      generateToken: 'Buat Token Agent',
      skipForNow: 'Lewati Sementara',
      completeTitle: 'MeetoOps Siap Digunakan',
      completeDesc: 'Akun administrator, database SQLite WAL, dan brankas enkripsi AEAD telah berhasil diinisialisasi.',
      goToDashboard: 'Buka Dasbor',
    },
    login: {
      title: 'Masuk ke MeetoOps',
      subtitle: 'Masukkan kredensial administrator atau tim untuk mengakses pusat kendali.',
      identifier: 'Email atau Username',
      password: 'Kata Sandi',
      signIn: 'Masuk',
    },
    dashboard: {
      title: 'Dasbor Infrastruktur',
      subtitle: 'Telemetri host langsung, status Docker lokal, dan aktivitas audit keamanan terbaru.',
      refresh: 'Muat Ulang',
      serversTotal: 'Server',
      containersTotal: 'Kontainer',
      hostCpu: 'CPU Host',
      hostMemory: 'Memori Host',
      hostDisk: 'Disk Host',
      hostUptime: 'Uptime Sistem',
      dockerEngine: 'Docker Engine Lokal',
      recentAudit: 'Aktivitas Audit Terbaru',
      noEvents: 'Belum ada aktivitas audit yang tercatat.',
    },
    settings: {
      title: 'Pengaturan Aplikasi',
      subtitle: 'Kelola preferensi runtime, port server, pengguna, dan kebijakan keamanan langsung dari UI tanpa mengedit file .env.',
      tabGeneral: 'Umum & Jaringan',
      tabUsers: 'Pengguna & Peran',
      tabSecurity: 'Keamanan & Vault',
      tabAudit: 'Log Audit',
      appName: 'Nama Aplikasi',
      appUrl: 'URL Publik Aplikasi',
      language: 'Bahasa Antarmuka',
      timezone: 'Zona Waktu',
      dateFormat: 'Format Tanggal',
      timeFormat: 'Format Waktu',
      monitoringInterval: 'Interval Pemantauan (detik)',
      networkPortTitle: 'Port Web & API',
      networkPortDesc: 'Periksa ketersediaan port langsung pada OS sebelum menyimpan agar tidak terjadi bentrok port.',
      listenPort: 'Port Terkonfigurasi',
      activePort: 'Port Aktif Saat Ini',
      checkPortBtn: 'Cek Ketersediaan',
      restartNotice: 'Port diperbarui di database. Restart server/container MeetoOps (serta mapping port Docker jika berjalan di container) agar port baru aktif.',
      saveSettings: 'Simpan Pengaturan',
      savedToast: 'Pengaturan berhasil disimpan.',
      vaultStatus: 'Brankas Rahasia AES-256-GCM Aktif',
      vaultFingerprint: 'Sidik Jari Master Key',
      addUser: 'Tambah Pengguna',
    },
  },
};
