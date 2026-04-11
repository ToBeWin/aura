import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type State =
  | "initializing"
  | "idle"
  | "listening"
  | "processing"
  | "typing"
  | "done"
  | "error";

type OutputMode = "note";
type ProviderMode = "local" | "cloud";
type LLMCloudProvider =
  | "openai"
  | "anthropic"
  | "gemini"
  | "deepseek"
  | "qwen"
  | "glm"
  | "kimi"
  | "minimax"
  | "openrouter"
  | "custom";
type ASRCloudProvider = "openai" | "groq" | "deepgram" | "assemblyai" | "azure" | "google" | "custom";
type Locale = "zh" | "en";
type DashboardSection = "overview" | "general" | "speech" | "refine" | "wake" | "history";

interface VoiceResult {
  transcript: string;
  text: string;
  processing_time_ms: number;
  confidence: number;
  applied_rules: Array<{ rule_type: string; from: string; to: string }>;
  output_mode: OutputMode;
}

interface PasteResult {
  text: string;
  delivered: boolean;
  copiedToClipboard: boolean;
  message: string;
}

interface AudioInputStatus {
  deviceName: string;
  sampleRate: number;
  channels: number;
  preferredDeviceName?: string | null;
  usingPreferredDevice: boolean;
}

interface DiagnosticStatus {
  ready: boolean;
  title: string;
  detail: string;
}

interface EnvironmentDiagnostics {
  speech: DiagnosticStatus;
  refine: DiagnosticStatus;
  delivery: DiagnosticStatus;
}

interface ASRProviderSettings {
  provider: ProviderMode;
  localModel: string;
  cloudProvider: ASRCloudProvider;
  cloudBaseUrl: string;
  cloudApiKey: string;
  cloudModel: string;
  language: string;
}

interface LLMProviderSettings {
  provider: ProviderMode;
  localModel: string;
  localBaseUrl: string;
  cloudProvider: LLMCloudProvider;
  cloudBaseUrl: string;
  cloudApiKey: string;
  cloudModel: string;
  cloudEmbeddingModel: string;
}

interface ProviderSettings {
  asr: ASRProviderSettings;
  llm: LLMProviderSettings;
  locale: Locale;
  wakeWordEnabled: boolean;
  wakeWordPhrase: string;
}

interface ProviderSettingsEnvelope {
  settings: ProviderSettings;
  settingsPath: string;
}

interface HistoryItem {
  id: string;
  transcript: string;
  refined: string;
  delivered: boolean;
  timestamp: number;
}

interface LocalASRModelStatus {
  modelName: string;
  downloaded: boolean;
  path: string;
  sizeMb: number;
  suggestedDownloadMb: number;
}

interface LocalLLMModelCatalog {
  models: string[];
  reachable: boolean;
}

const LOCALE_STORAGE_KEY = "aura_locale";

const UI = {
  zh: {
    statuses: {
      noSpeech: "没有听到清晰语音，请靠近麦克风再试一次",
      recordingShort: "录音太短了，请至少连续说 2 秒",
      microphoneDenied: "没有麦克风权限，请先允许 Aura 使用麦克风",
      asrNotReady: "语音识别还没准备好，请稍后再试",
      auraNotReady: "Aura 还没初始化完成，请稍后再试",
      cloudLlmMissing: "云端润色缺少 API Key",
      cloudAsrMissing: "云端转写缺少 API Key",
      initFailed: "初始化失败，请重启 Aura 再试一次",
      genericError: "出了点问题，请再试一次",
      processing: "正在润色",
      typing: "正在输入",
      saved: "设置已保存",
      downloaded: (name: string) => `已下载 ${name}`,
      pasted: "已输入",
      copied: "已复制到剪贴板",
      accessibilityPrompt: "未开启辅助功能权限，已复制到剪贴板",
      cancel: "取消",
      checkingInput: "正在检测输入设备...",
    },
    dashboard: {
      title: "Aura 控制台",
      subtitle: "快捷键负责执行，主界面负责设置、模型管理和最近记录。",
      overviewSection: "首页",
      overviewTitle: "欢迎回来",
      overviewDescription: "Aura 已准备就绪。你可以从这里快速确认当前状态，再用快捷键开始一次语音输入。",
      capsuleReady: "胶囊已就绪",
      capsuleReadyNote: "按下 Option + Shift + Space 即可唤起胶囊，开始录音，再按一次结束并自动处理。",
      currentSetup: "当前配置",
      recentActivity: "最近活动",
      diagnostics: "环境诊断",
      refreshDiagnostics: "刷新诊断",
      refreshingDiagnostics: "刷新中...",
      openAccessibility: "打开辅助功能设置",
      speechProvider: "语音识别",
      refineProvider: "润色模型",
      wakeWordCard: "唤醒词",
      active: "已启用",
      inactive: "未启用",
      lastResult: "最近一条结果",
      noRecentActivity: "还没有最近记录。你可以先按快捷键说一句话试试看。",
      ready: "已就绪",
      needsAttention: "需处理",
      hotkey: "快捷键",
      language: "语言",
      speechLanguage: "识别语言",
      speechLanguageAuto: "自动识别",
      speechLanguageZh: "中文（简体）",
      speechLanguageEn: "英文",
      generalSection: "通用",
      speechSection: "语音识别",
      refineSection: "润色模型",
      wakeSection: "唤醒词",
      historySection: "最近记录",
      historyPagination: (page: number, total: number) => `第 ${page} / ${total} 页`,
      prevPage: "上一页",
      nextPage: "下一页",
      local: "本地",
      cloud: "云端",
      localModel: "本地模型",
      provider: "提供商",
      apiBaseUrl: "API Base URL",
      model: "模型",
      apiKey: "API Key",
      llmProviderHint:
        "当前优先支持 OpenAI、Anthropic、Gemini、DeepSeek、Qwen、GLM、Kimi、Minimax，以及兼容 OpenAI Chat Completions 的网关。",
      asrProviderHint: "当前优先支持 OpenAI、Groq、Deepgram、AssemblyAI、Azure Speech、Google Speech-to-Text，以及兼容 OpenAI transcription 接口的自定义网关。",
      providerOpenAI: "OpenAI",
      providerAnthropic: "Anthropic",
      providerGemini: "Gemini",
      providerDeepSeek: "DeepSeek",
      providerQwen: "Qwen",
      providerGlm: "GLM",
      providerKimi: "Kimi",
      providerMinimax: "Minimax",
      providerOpenRouter: "OpenRouter",
      providerGroq: "Groq",
      providerDeepgram: "Deepgram",
      providerAssemblyAI: "AssemblyAI",
      providerAzure: "Azure Speech",
      providerGoogle: "Google Speech-to-Text",
      providerCustom: "自定义兼容接口",
      recommendedModels: "推荐模型",
      customModel: "自定义",
      ollamaUrl: "Ollama 地址",
      recommended: "推荐",
      note: "说明",
      downloadedLocal: "已下载到本地",
      notDownloaded: "尚未下载",
      estimatedDownload: (mb: number) => `预计下载 ${mb} MB，到 ~/.aura/models`,
      installerHint: "安装包不会内置 Whisper，只有在你选择本地运行时才下载。",
      preparingDownload: "准备下载...",
      redownload: "重新下载",
      downloadModel: "下载模型",
      refreshModels: "刷新模型列表",
      modelCatalogOffline: "当前无法连接本地 Ollama，先显示推荐模型。",
      saving: "保存中...",
      saveSettings: "保存设置",
      settingsFile: "配置文件",
      emptyHistory: "这里会显示最近几次转写和润色结果。",
      delivered: "已输入",
      copied: "已复制",
      generalTitle: "通用设置",
      generalDescription: "这里放 Aura 的全局偏好，不和模型配置混在一起。",
      interfaceLanguage: "界面语言",
      activeInput: "当前输入设备",
      wakeTitle: "唤醒词",
      wakeDescription: "把唤醒词单独放一页，避免和模型配置混淆。",
      wakeEnabled: "启用唤醒词",
      wakePhrase: "唤醒词内容",
      wakeHint: "当前先保存配置，实时监听链路下一步继续接入。",
    },
    modelHints: {
      "whisper-tiny": "最低资源占用，适合老机器或临时体验。",
      "whisper-base": "推荐默认选项，速度和识别率更均衡。",
      "whisper-small": "更高精度，但下载和运行成本会明显增加。",
      "whisper-medium": "适合高性能电脑，响应会比 Base 慢。",
      "whisper-large-v3": "最高精度路线，只建议高配机器使用。",
    },
  },
  en: {
    statuses: {
      noSpeech: "No clear speech detected. Please move closer to the microphone and try again.",
      recordingShort: "Recording was too short. Please speak continuously for at least 2 seconds.",
      microphoneDenied: "Microphone access is required. Please allow Aura to use the microphone.",
      asrNotReady: "Speech recognition is not ready yet. Please try again shortly.",
      auraNotReady: "Aura is still initializing. Please try again in a moment.",
      cloudLlmMissing: "Cloud refine provider is missing an API key.",
      cloudAsrMissing: "Cloud transcription provider is missing an API key.",
      initFailed: "Initialization failed. Please restart Aura and try again.",
      genericError: "Something went wrong. Please try again.",
      processing: "Refining",
      typing: "Typing",
      saved: "Settings saved",
      downloaded: (name: string) => `Downloaded ${name}`,
      pasted: "Inserted",
      copied: "Copied to clipboard",
      accessibilityPrompt: "Accessibility permission is required for auto-paste. Copied to clipboard.",
      cancel: "Cancel",
      checkingInput: "Checking audio input...",
    },
    dashboard: {
      title: "Aura Console",
      subtitle: "The hotkey is for execution. This window is for settings, model management, and recent history.",
      overviewSection: "Home",
      overviewTitle: "Welcome back",
      overviewDescription: "Aura is ready. Use this page to confirm the current setup, then trigger the capsule with the hotkey.",
      capsuleReady: "Capsule ready",
      capsuleReadyNote: "Press Option + Shift + Space to summon the capsule, start speaking, then press again to finish and process.",
      currentSetup: "Current setup",
      recentActivity: "Recent activity",
      diagnostics: "Environment diagnostics",
      refreshDiagnostics: "Refresh diagnostics",
      refreshingDiagnostics: "Refreshing...",
      openAccessibility: "Open Accessibility settings",
      speechProvider: "Speech recognition",
      refineProvider: "Refine model",
      wakeWordCard: "Wake word",
      active: "Active",
      inactive: "Inactive",
      lastResult: "Latest result",
      noRecentActivity: "No recent activity yet. Try speaking once with the hotkey.",
      ready: "Ready",
      needsAttention: "Needs attention",
      hotkey: "Hotkey",
      language: "Language",
      speechLanguage: "Recognition language",
      speechLanguageAuto: "Auto detect",
      speechLanguageZh: "Chinese (Simplified)",
      speechLanguageEn: "English",
      generalSection: "General",
      speechSection: "Speech recognition",
      refineSection: "Refine model",
      wakeSection: "Wake Word",
      historySection: "Recent history",
      historyPagination: (page: number, total: number) => `Page ${page} of ${total}`,
      prevPage: "Previous",
      nextPage: "Next",
      local: "Local",
      cloud: "Cloud",
      localModel: "Local model",
      provider: "Provider",
      apiBaseUrl: "API Base URL",
      model: "Model",
      apiKey: "API Key",
      llmProviderHint:
        "Officially supported: OpenAI, Anthropic, Gemini, DeepSeek, Qwen, GLM, Kimi, Minimax, and gateways compatible with OpenAI Chat Completions.",
      asrProviderHint: "Right now Aura officially supports OpenAI, Groq, Deepgram, AssemblyAI, Azure Speech, Google Speech-to-Text, and custom gateways that implement the OpenAI transcription API.",
      providerOpenAI: "OpenAI",
      providerAnthropic: "Anthropic",
      providerGemini: "Gemini",
      providerDeepSeek: "DeepSeek",
      providerQwen: "Qwen",
      providerGlm: "GLM",
      providerKimi: "Kimi",
      providerMinimax: "Minimax",
      providerOpenRouter: "OpenRouter",
      providerGroq: "Groq",
      providerDeepgram: "Deepgram",
      providerAssemblyAI: "AssemblyAI",
      providerAzure: "Azure Speech",
      providerGoogle: "Google Speech-to-Text",
      providerCustom: "Custom compatible API",
      recommendedModels: "Recommended models",
      customModel: "Custom",
      ollamaUrl: "Ollama URL",
      recommended: "Recommended",
      note: "Note",
      downloadedLocal: "Downloaded locally",
      notDownloaded: "Not downloaded yet",
      estimatedDownload: (mb: number) => `Estimated download ${mb} MB to ~/.aura/models`,
      installerHint: "Whisper is not bundled in the installer. It downloads only when local ASR is selected.",
      preparingDownload: "Preparing download...",
      redownload: "Download again",
      downloadModel: "Download model",
      refreshModels: "Refresh model list",
      modelCatalogOffline: "Ollama is not reachable right now. Showing recommended models instead.",
      saving: "Saving...",
      saveSettings: "Save settings",
      settingsFile: "Settings file",
      emptyHistory: "Recent transcripts and refined results will appear here.",
      delivered: "Inserted",
      copied: "Copied",
      generalTitle: "General settings",
      generalDescription: "Keep Aura's global preferences separate from model configuration.",
      interfaceLanguage: "Interface language",
      activeInput: "Active input device",
      wakeTitle: "Wake word",
      wakeDescription: "Wake-word preferences live on their own page so they do not get mixed up with model settings.",
      wakeEnabled: "Enable wake word",
      wakePhrase: "Wake word phrase",
      wakeHint: "This currently saves the preference only. Live wake-word listening is the next step.",
    },
    modelHints: {
      "whisper-tiny": "Lowest resource use. Best for older machines or quick evaluation.",
      "whisper-base": "Recommended default. A balanced choice for speed and recognition quality.",
      "whisper-small": "Higher accuracy, but noticeably heavier to download and run.",
      "whisper-medium": "A better fit for powerful computers. Slower than Base.",
      "whisper-large-v3": "Highest-accuracy route. Recommended only for high-end machines.",
    },
  },
} as const;

function readStoredLocale(): Locale {
  try {
    const raw = window.localStorage.getItem(LOCALE_STORAGE_KEY);
    if (raw === "zh" || raw === "en") return raw;
  } catch {
    // ignore
  }
  return "en";
}

function useAuraLocale() {
  const [locale, setLocaleState] = useState<Locale>(() => readStoredLocale());

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === LOCALE_STORAGE_KEY && (event.newValue === "zh" || event.newValue === "en")) {
        setLocaleState(event.newValue);
      }
    };
    const onLocaleChange = (event: Event) => {
      const next = (event as CustomEvent<Locale>).detail;
      if (next === "zh" || next === "en") {
        setLocaleState(next);
      }
    };
    window.addEventListener("storage", onStorage);
    window.addEventListener("aura:locale-changed", onLocaleChange as EventListener);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("aura:locale-changed", onLocaleChange as EventListener);
    };
  }, []);

  const setLocale = (next: Locale) => {
    setLocaleState(next);
    try {
      window.localStorage.setItem(LOCALE_STORAGE_KEY, next);
    } catch {
      // ignore
    }
    window.dispatchEvent(new CustomEvent("aura:locale-changed", { detail: next }));
  };

  return { locale, setLocale, ui: UI[locale] };
}

function getLocalAsrChoices(locale: Locale) {
  const modelHints = UI[locale].modelHints;
  return [
    { value: "whisper-tiny", label: "Whisper Tiny", hint: modelHints["whisper-tiny"] },
    { value: "whisper-base", label: "Whisper Base", hint: modelHints["whisper-base"] },
    { value: "whisper-small", label: "Whisper Small", hint: modelHints["whisper-small"] },
    { value: "whisper-medium", label: "Whisper Medium", hint: modelHints["whisper-medium"] },
    { value: "whisper-large-v3", label: "Whisper Large v3", hint: modelHints["whisper-large-v3"] },
  ];
}

function getDefaultLlmCloudBaseUrl(provider: LLMCloudProvider) {
  switch (provider) {
    case "anthropic":
      return "https://api.anthropic.com/v1";
    case "gemini":
      return "https://generativelanguage.googleapis.com/v1beta";
    case "deepseek":
      return "https://api.deepseek.com/v1";
    case "qwen":
      return "https://dashscope.aliyuncs.com/compatible-mode/v1";
    case "glm":
      return "https://open.bigmodel.cn/api/paas/v4";
    case "kimi":
      return "https://api.moonshot.cn/v1";
    case "minimax":
      return "https://api.minimax.chat/v1";
    case "openrouter":
      return "https://openrouter.ai/api/v1";
    case "custom":
      return "https://api.openai.com/v1";
    case "openai":
    default:
      return "https://api.openai.com/v1";
  }
}

function getDefaultLlmCloudModel(provider: LLMCloudProvider) {
  switch (provider) {
    case "anthropic":
      return "claude-3-5-sonnet-latest";
    case "gemini":
      return "gemini-1.5-pro";
    case "deepseek":
      return "deepseek-chat";
    case "qwen":
      return "qwen-plus";
    case "glm":
      return "glm-4";
    case "kimi":
      return "moonshot-v1-32k";
    case "minimax":
      return "abab6.5s";
    case "openrouter":
      return "openai/gpt-4.1-mini";
    case "custom":
      return "gpt-4.1-mini";
    case "openai":
    default:
      return "gpt-4.1-mini";
  }
}

function getDefaultLlmEmbeddingModel(_provider: LLMCloudProvider) {
  return "text-embedding-3-small";
}

function getDefaultAsrCloudBaseUrl(provider: ASRCloudProvider) {
  switch (provider) {
    case "groq":
      return "https://api.groq.com/openai/v1";
    case "deepgram":
      return "https://api.deepgram.com/v1";
    case "assemblyai":
      return "https://api.assemblyai.com/v2";
    case "azure":
      return "https://eastus.stt.speech.microsoft.com";
    case "google":
      return "https://speech.googleapis.com/v1";
    case "custom":
      return "https://api.openai.com/v1";
    case "openai":
    default:
      return "https://api.openai.com/v1";
  }
}

function getDefaultAsrCloudModel(provider: ASRCloudProvider) {
  switch (provider) {
    case "groq":
      return "whisper-large-v3-turbo";
    case "deepgram":
      return "nova-2";
    case "assemblyai":
      return "best";
    case "azure":
      return "latest";
    case "google":
      return "latest_long";
    case "custom":
      return "gpt-4o-mini-transcribe";
    case "openai":
    default:
      return "gpt-4o-mini-transcribe";
  }
}

const ASR_CLOUD_MODEL_OPTIONS: Record<ASRCloudProvider, { value: string; label: string }[]> = {
  openai: [
    { value: "gpt-4o-mini-transcribe", label: "gpt-4o-mini-transcribe" },
    { value: "gpt-4o-transcribe", label: "gpt-4o-transcribe" },
    { value: "whisper-1", label: "whisper-1" },
  ],
  groq: [
    { value: "whisper-large-v3-turbo", label: "whisper-large-v3-turbo" },
    { value: "whisper-large-v3", label: "whisper-large-v3" },
    { value: "distil-whisper-large-v3-en", label: "distil-whisper-large-v3-en" },
  ],
  deepgram: [
    { value: "nova-2", label: "nova-2" },
    { value: "nova-2-general", label: "nova-2-general" },
    { value: "nova-2-meeting", label: "nova-2-meeting" },
    { value: "nova-2-phonecall", label: "nova-2-phonecall" },
  ],
  assemblyai: [
    { value: "best", label: "best" },
  ],
  azure: [
    { value: "latest", label: "latest" },
  ],
  google: [
    { value: "latest_long", label: "latest_long" },
    { value: "latest_short", label: "latest_short" },
  ],
  custom: [
    { value: "gpt-4o-mini-transcribe", label: "gpt-4o-mini-transcribe" },
  ],
};

const LLM_CLOUD_MODEL_OPTIONS: Record<LLMCloudProvider, { value: string; label: string }[]> = {
  openai: [
    { value: "gpt-4.1", label: "gpt-4.1" },
    { value: "gpt-4.1-mini", label: "gpt-4.1-mini" },
    { value: "gpt-4o", label: "gpt-4o" },
    { value: "gpt-4o-mini", label: "gpt-4o-mini" },
  ],
  anthropic: [
    { value: "claude-3-5-sonnet-latest", label: "claude-3-5-sonnet-latest" },
    { value: "claude-3-5-haiku-latest", label: "claude-3-5-haiku-latest" },
    { value: "claude-3-opus-20240229", label: "claude-3-opus-20240229" },
  ],
  gemini: [
    { value: "gemini-1.5-pro", label: "gemini-1.5-pro" },
    { value: "gemini-1.5-flash", label: "gemini-1.5-flash" },
    { value: "gemini-2.0-flash", label: "gemini-2.0-flash" },
  ],
  deepseek: [
    { value: "deepseek-chat", label: "deepseek-chat" },
    { value: "deepseek-reasoner", label: "deepseek-reasoner" },
  ],
  qwen: [
    { value: "qwen-turbo", label: "qwen-turbo" },
    { value: "qwen-plus", label: "qwen-plus" },
    { value: "qwen-max", label: "qwen-max" },
  ],
  glm: [
    { value: "glm-4", label: "glm-4" },
    { value: "glm-4-plus", label: "glm-4-plus" },
  ],
  kimi: [
    { value: "moonshot-v1-8k", label: "moonshot-v1-8k" },
    { value: "moonshot-v1-32k", label: "moonshot-v1-32k" },
    { value: "moonshot-v1-128k", label: "moonshot-v1-128k" },
  ],
  minimax: [
    { value: "abab6.5s", label: "abab6.5s" },
    { value: "abab6.5-chat", label: "abab6.5-chat" },
  ],
  openrouter: [
    { value: "openai/gpt-4.1-mini", label: "openai/gpt-4.1-mini" },
    { value: "anthropic/claude-3.5-sonnet", label: "anthropic/claude-3.5-sonnet" },
    { value: "google/gemini-1.5-pro", label: "google/gemini-1.5-pro" },
  ],
  custom: [
    { value: "gpt-4.1-mini", label: "gpt-4.1-mini" },
  ],
};

const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  asr: {
    provider: "local",
    localModel: "whisper-base",
    cloudProvider: "openai",
    cloudBaseUrl: "https://api.openai.com/v1",
    cloudApiKey: "",
    cloudModel: "gpt-4o-mini-transcribe",
    language: "auto",
  },
  llm: {
    provider: "local",
    localModel: "qwen3.5:2b",
    localBaseUrl: "http://localhost:11434",
    cloudProvider: "openai",
    cloudBaseUrl: "https://api.openai.com/v1",
    cloudApiKey: "",
    cloudModel: "gpt-4.1-mini",
    cloudEmbeddingModel: "text-embedding-3-small",
  },
  locale: "en",
  wakeWordEnabled: false,
  wakeWordPhrase: "Aura",
};

function normalizeStatusMessage(raw: unknown, locale: Locale) {
  const message = String(raw ?? "").trim();
  const upper = message.toUpperCase();
  const t = UI[locale].statuses;

  if (upper.includes("NO SPEECH DETECTED")) return t.noSpeech;
  if (upper.includes("RECORDING TOO SHORT")) return t.recordingShort;
  if (upper.includes("MICROPHONE ACCESS DENIED")) return t.microphoneDenied;
  if (upper.includes("ASR NOT READY")) return t.asrNotReady;
  if (upper.includes("AURA NOT READY")) return t.auraNotReady;
  if (upper.includes("CLOUD LLM API KEY IS MISSING")) return t.cloudLlmMissing;
  if (upper.includes("CLOUD ASR API KEY IS MISSING")) return t.cloudAsrMissing;
  if (upper.includes("INIT FAILED")) return t.initFailed;

  return message || t.genericError;
}

async function initializeAura() {
  await invoke("initialize_aura", {
    modelName: "qwen3.5:2b",
    dbPath: "",
    vectorDbPath: "",
  });
}

function CapsuleApp() {
  const { locale, setLocale, ui } = useAuraLocale();
  const [state, setState] = useState<State>("initializing");
  const [audioLevel, setAudioLevel] = useState(0);
  const [statusMsg, setStatusMsg] = useState("");
  const [accessibilityNeeded, setAccessibilityNeeded] = useState(false);
  const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);
  const outputMode: OutputMode = "note";

  const recorder = useRef<MediaRecorder | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const chunks = useRef<Blob[]>([]);
  const animFrame = useRef(0);
  const analyser = useRef<AnalyserNode | null>(null);
  const audioContext = useRef<AudioContext | null>(null);
  const globalCaptureActive = useRef(false);
  const hideOnIdle = useRef(false);
  const appWindow = useRef(getCurrentWindow());
  const processingStartedAt = useRef<number | null>(null);
  const cancelRequested = useRef(false);
  const pipelineRunId = useRef(0);
  const stateRef = useRef<State>("initializing");
  const timing = useRef({
    minProcessingMs: 600,
    doneHoldMs: 900,
    errorHoldMs: 1600,
  });

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    void (async () => {
      try {
        const providerEnvelope = await invoke<ProviderSettingsEnvelope>("get_provider_settings");
        const nextLocale = providerEnvelope.settings.locale;
        if (nextLocale === "zh" || nextLocale === "en") {
          setLocale(nextLocale);
        }
      } catch {
        // ignore
      }
    })();
  }, []);

  useEffect(() => {
    try {
      const readInt = (key: string, fallback: number) => {
        const raw = window.localStorage.getItem(key);
        const value = raw ? Number(raw) : NaN;
        return Number.isFinite(value) && value > 0 ? value : fallback;
      };
      timing.current = {
        minProcessingMs: readInt("aura_processing_min_ms", timing.current.minProcessingMs),
        doneHoldMs: readInt("aura_done_hold_ms", timing.current.doneHoldMs),
        errorHoldMs: readInt("aura_error_hold_ms", timing.current.errorHoldMs),
      };
    } catch {
      // ignore
    }

    void (async () => {
      try {
        await initializeAura();
        setState("idle");
      } catch (error) {
        setState("error");
        setStatusMsg(normalizeStatusMessage(`Init failed: ${error}`, locale));
      }
    })();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.repeat) return;

      if (event.code === "Escape") {
        event.preventDefault();
        if (
          stateRef.current === "listening" ||
          stateRef.current === "processing" ||
          stateRef.current === "typing"
        ) {
          void cancelCurrentFlow();
        }
        return;
      }

      if (event.code !== "Space" || !event.altKey || !event.shiftKey) return;

      if (
        stateRef.current === "idle" ||
        stateRef.current === "done" ||
        stateRef.current === "error"
      ) {
        event.preventDefault();
        void startRecording("local");
      } else if (stateRef.current === "listening") {
        event.preventDefault();
        void stopRecording("local");
      }
    };

    window.addEventListener("keydown", onKeyDown);

    let unlistenStarted: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    let unlistenLevel: (() => void) | undefined;

    void (async () => {
      unlistenStarted = await listen("recording_started", async () => {
        globalCaptureActive.current = true;
        hideOnIdle.current = true;
        void appWindow.current.show();
        await startRecording("global");
      });

      unlistenDone = await listen("voice_done", async () => {
        await stopRecording("global");
        globalCaptureActive.current = false;
      });

      unlistenLevel = await listen<{ level?: number }>("recording_level", (event) => {
        if (stream.current) return;
        if (!globalCaptureActive.current && stateRef.current !== "listening") return;
        const level = Number(event.payload?.level ?? 0);
        setAudioLevel(Number.isFinite(level) ? Math.max(0, Math.min(1, level)) : 0);
      });
    })();

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      unlistenStarted?.();
      unlistenDone?.();
      unlistenLevel?.();
      stopMic();
    };
  }, []);

  async function closeCapsule() {
    setAudioLevel(0);
    setAccessibilityNeeded(false);
    hideOnIdle.current = true;
    try {
      await invoke("hide_capsule_window");
    } catch {
      try {
        await appWindow.current.hide();
      } catch {
        // ignore hide failures in dev
      }
    }
  }

  async function cancelCurrentFlow() {
    cancelRequested.current = true;
    pipelineRunId.current += 1;
    setStatusMsg("");
    setAccessibilityNeeded(false);

    if (recorder.current && recorder.current.state !== "inactive") {
      await stopRecording("local");
      return;
    }

    setState("idle");
    setAudioLevel(0);
    await closeCapsule();
  }

  async function startRecording(source: "local" | "global") {
    if (recorder.current && recorder.current.state !== "inactive") return;

    cancelRequested.current = false;
    hideOnIdle.current = true;
    setStatusMsg("");
    setAccessibilityNeeded(false);
    setState("listening");
    chunks.current = [];

    try {
      const mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      stream.current = mediaStream;
      const context = new AudioContext();
      if (context.state === "suspended") {
        await context.resume();
      }
      audioContext.current = context;
      const mediaSource = context.createMediaStreamSource(mediaStream);
      const nextAnalyser = context.createAnalyser();
      nextAnalyser.fftSize = 512;
      nextAnalyser.smoothingTimeConstant = 0.78;
      mediaSource.connect(nextAnalyser);
      analyser.current = nextAnalyser;

      const mediaRecorder = new MediaRecorder(mediaStream);
      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          chunks.current.push(event.data);
        }
      };
      mediaRecorder.onstop = () => {
        const blob = new Blob(chunks.current, { type: "audio/webm" });
        stopMic();

        if (cancelRequested.current) {
          setState("idle");
          void closeCapsule();
          return;
        }

        if (blob.size > 100) {
          void handleAudio(blob, source);
        } else {
          setState("error");
          setStatusMsg(ui.statuses.recordingShort);
          setTimeout(() => {
            setState("idle");
            setStatusMsg("");
            void closeCapsule();
          }, 1400);
        }
      };
      recorder.current = mediaRecorder;
      mediaRecorder.start();
      meter();
    } catch {
      setState("error");
      setStatusMsg(normalizeStatusMessage("Microphone access denied", locale));
    }
  }

  async function stopRecording(_source: "local" | "global") {
    const currentRecorder = recorder.current;
    if (!currentRecorder || currentRecorder.state === "inactive") {
      stopMic();
      return;
    }
    currentRecorder.stop();
  }

  function stopMic() {
    cancelAnimationFrame(animFrame.current);
    stream.current?.getTracks().forEach((track) => track.stop());
    void audioContext.current?.close();
    audioContext.current = null;
    stream.current = null;
    analyser.current = null;
    recorder.current = null;
    setAudioLevel(0);
  }

  function meter() {
    if (!analyser.current) return;
    const data = new Uint8Array(analyser.current.fftSize);
    const tick = () => {
      analyser.current?.getByteTimeDomainData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i += 1) {
        const normalized = (data[i] - 128) / 128;
        sum += normalized * normalized;
      }
      const rms = Math.sqrt(sum / data.length);
      const boosted = Math.max(0, rms - 0.008) * 16;
      setAudioLevel(Math.min(1, boosted));
      animFrame.current = requestAnimationFrame(tick);
    };
    tick();
  }

  async function runVoicePipeline(source: "local" | "global", blob?: Blob) {
    const runId = ++pipelineRunId.current;
    setState("processing");
    processingStartedAt.current = Date.now();

    try {
      let result: VoiceResult;

      if (blob) {
        const buffer = await blob.arrayBuffer();
        result = await invoke<VoiceResult>("process_voice", {
          request: {
            audioData: Array.from(new Uint8Array(buffer)),
            audioFormat: "webm",
            outputMode,
          },
        });
      } else {
        result = await invoke<VoiceResult>("process_voice", {
          request: { outputMode },
        });
      }

      if (cancelRequested.current || runId !== pipelineRunId.current) return;
      await deliverDraft(result, source);
    } catch (error: unknown) {
      if (cancelRequested.current || runId !== pipelineRunId.current) return;
      const message = normalizeStatusMessage(
        error instanceof Error ? error.message : String(error),
        locale,
      );
      setState("error");
      setStatusMsg(message.slice(0, 80));
      setTimeout(() => {
        setState("idle");
        setStatusMsg("");
        void closeCapsule();
      }, timing.current.errorHoldMs);
    }
  }

  async function handleAudio(blob: Blob, source: "local" | "global") {
    await runVoicePipeline(source, blob);
  }

  async function deliverDraft(result: VoiceResult, _source: "local" | "global") {
    const runId = pipelineRunId.current;
    setState("typing");

    try {
      const pasteResult = await invoke<PasteResult>("type_text", { text: result.text });
      if (cancelRequested.current || runId !== pipelineRunId.current) return;

      const elapsed = processingStartedAt.current
        ? Date.now() - processingStartedAt.current
        : 0;
      const waitFor = Math.max(0, timing.current.minProcessingMs - elapsed);
      const delivered = pasteResult.delivered;

      try {
        await invoke("record_history_entry", {
          request: {
            transcript: result.transcript,
            refined: result.text,
            delivered,
          },
        });
      } catch (historyError) {
        console.warn("[Aura] Failed to persist history entry", historyError);
      }

      const needsAccessibility = !delivered && isMac;
      const doneMessage = delivered
        ? ui.statuses.pasted
        : normalizeStatusMessage(pasteResult.message || ui.statuses.copied, locale);

      setTimeout(() => {
        if (!delivered && needsAccessibility) {
          setAccessibilityNeeded(true);
          setState("error");
          setStatusMsg(
            normalizeStatusMessage(pasteResult.message || ui.statuses.accessibilityPrompt, locale),
          );
          setTimeout(() => {
            setState("idle");
            setStatusMsg("");
            void closeCapsule();
          }, timing.current.errorHoldMs);
          return;
        }

        setState("done");
        setStatusMsg(doneMessage);
        setTimeout(() => {
          setState("idle");
          setStatusMsg("");
          void closeCapsule();
        }, timing.current.doneHoldMs);
      }, waitFor);
    } catch (error: unknown) {
      if (cancelRequested.current || runId !== pipelineRunId.current) return;
      const message = normalizeStatusMessage(
        error instanceof Error ? error.message : String(error),
        locale,
      );
      const elapsed = processingStartedAt.current
        ? Date.now() - processingStartedAt.current
        : 0;
      const waitFor = Math.max(0, timing.current.minProcessingMs - elapsed);

      setTimeout(() => {
        setState("error");
        setStatusMsg(message.slice(0, 80));
        setTimeout(() => {
          setState("idle");
          setStatusMsg("");
          void closeCapsule();
        }, timing.current.errorHoldMs);
      }, waitFor);
    }
  }

  const showStatusStrip =
    state === "processing" ||
    state === "typing" ||
    state === "error" ||
    state === "done" ||
    (!!statusMsg && state === "idle");
  const listeningActive = audioLevel > 0.035;

  return (
    <div className={`app ${state}`}>
      <div className="shell">
        <div className="pill">
          <div className="pill-core" aria-hidden="true">
            <div className={`dot ${state}`} />

            {state === "idle" && (
              <div className="voice-dots idle-dots">
                <span className="voice-dot" />
                <span className="voice-dot" />
                <span className="voice-dot" />
              </div>
            )}

            {state === "listening" && (
              <div className={`voice-dots ${listeningActive ? "is-speaking" : "is-resting"}`}>
                {[0.06, 0.12, 0.19, 0.28].map((threshold, index) => {
                  const intensity = Math.max(0, Math.min(1, (audioLevel - threshold) / 0.16));
                  return (
                  <span
                    key={index}
                    className={`voice-dot ${intensity > 0.05 ? "active" : ""}`}
                    style={{
                      opacity: 0.2 + intensity * 0.8,
                      transform: `scale(${0.9 + intensity * 0.5})`,
                    }}
                  />
                  );
                })}
              </div>
            )}

            {(state === "processing" || state === "typing" || state === "initializing") && (
              <div className={`orbit-dots ${state === "typing" ? "is-typing" : "is-processing"}`}>
                <span />
                <span />
                <span />
              </div>
            )}

            {state === "done" && <span className="state-glyph success">✓</span>}
            {state === "error" && <span className="state-glyph error">!</span>}
          </div>

          {(state === "listening" || state === "processing" || state === "typing") && (
            <div className="pill-actions">
              <button
                type="button"
                className="icon-button cancel"
                onPointerDown={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  void cancelCurrentFlow();
                }}
                aria-label={ui.statuses.cancel}
              >
                ×
              </button>
            </div>
          )}
        </div>

        {showStatusStrip && (
          <div className="status-strip">
            <span>
              {statusMsg ||
                (state === "processing"
                  ? ui.statuses.processing
                  : state === "typing"
                    ? ui.statuses.typing
                    : "")}
            </span>
            {accessibilityNeeded && (
              <button
                type="button"
                className="status-action"
                onClick={() => void invoke("open_accessibility_settings")}
              >
                {ui.dashboard.openAccessibility}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function DashboardApp() {
  const { locale, setLocale, ui } = useAuraLocale();
  const [dashboardReady, setDashboardReady] = useState(false);
  const [activeSection, setActiveSection] = useState<DashboardSection>("overview");
  const [statusMsg, setStatusMsg] = useState("");
  const [audioInputLabel, setAudioInputLabel] = useState("");
  const [providerSettings, setProviderSettings] = useState<ProviderSettings>(DEFAULT_PROVIDER_SETTINGS);
  const [settingsPath, setSettingsPath] = useState("");
  const [settingsSaving, setSettingsSaving] = useState(false);
  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]);
  const [historyPage, setHistoryPage] = useState(1);
  const [diagnostics, setDiagnostics] = useState<EnvironmentDiagnostics | null>(null);
  const [diagnosticsRefreshing, setDiagnosticsRefreshing] = useState(false);
  const [localAsrStatus, setLocalAsrStatus] = useState<LocalASRModelStatus | null>(null);
  const [localLlmModels, setLocalLlmModels] = useState<LocalLLMModelCatalog | null>(null);
  const [localLlmModelsRefreshing, setLocalLlmModelsRefreshing] = useState(false);
  const [downloadingAsr, setDownloadingAsr] = useState(false);
  const localAsrChoices = getLocalAsrChoices(locale);
  const selectedLocalAsr =
    localAsrChoices.find((choice) => choice.value === providerSettings.asr.localModel) ??
    localAsrChoices[1];
  const HISTORY_PAGE_SIZE = 8;
  const totalHistoryPages = Math.max(1, Math.ceil(historyItems.length / HISTORY_PAGE_SIZE));
  const clampedHistoryPage = Math.min(historyPage, totalHistoryPages);
  const pagedHistoryItems = historyItems.slice(
    (clampedHistoryPage - 1) * HISTORY_PAGE_SIZE,
    clampedHistoryPage * HISTORY_PAGE_SIZE,
  );
  const localLlmChoices = Array.from(
    new Set([
      providerSettings.llm.localModel,
      ...(localLlmModels?.models ?? [
        "qwen3.5:2b",
        "qwen2.5:7b",
        "llama3.2:3b",
        "gemma3:4b",
        "mistral:7b",
      ]),
    ]),
  );

  useEffect(() => {
    if (!statusMsg) return;
    const timer = window.setTimeout(() => {
      setStatusMsg("");
    }, 2200);
    return () => window.clearTimeout(timer);
  }, [statusMsg]);

  useEffect(() => {
    void (async () => {
      try {
        await initializeAura();
        const [providerEnvelope, history] = await Promise.all([
          invoke<ProviderSettingsEnvelope>("get_provider_settings"),
          invoke<HistoryItem[]>("get_history_entries"),
        ]);
        setProviderSettings(providerEnvelope.settings);
        if (providerEnvelope.settings.locale === "zh" || providerEnvelope.settings.locale === "en") {
          setLocale(providerEnvelope.settings.locale);
        }
        setSettingsPath(providerEnvelope.settingsPath);
        setHistoryItems(history);
        await refreshDiagnostics();
        await refreshLocalLlmModels();
        if (providerEnvelope.settings.asr.provider === "local") {
          const status = await invoke<LocalASRModelStatus>("get_local_asr_model_status", {
            request: { modelName: providerEnvelope.settings.asr.localModel },
          });
          setLocalAsrStatus(status);
        }
      } catch (error) {
        setStatusMsg(normalizeStatusMessage(`Init failed: ${error}`, locale));
      } finally {
        setDashboardReady(true);
      }
    })();

    let unlistenHistory: (() => void) | undefined;
    void (async () => {
      unlistenHistory = await listen<HistoryItem>("history_updated", (event) => {
        setHistoryItems((current) => [event.payload, ...current].slice(0, 50));
        setHistoryPage(1);
      });
    })();

    return () => {
      unlistenHistory?.();
    };
  }, []);

  useEffect(() => {
    if (providerSettings.asr.provider !== "local") {
      setLocalAsrStatus(null);
      return;
    }

    void (async () => {
      try {
        const status = await invoke<LocalASRModelStatus>("get_local_asr_model_status", {
          request: { modelName: providerSettings.asr.localModel },
        });
        setLocalAsrStatus(status);
      } catch {
        setLocalAsrStatus(null);
      }
    })();
  }, [providerSettings.asr.provider, providerSettings.asr.localModel]);

  useEffect(() => {
    if (providerSettings.llm.provider !== "local") return;
    void refreshLocalLlmModels();
  }, [providerSettings.llm.provider, providerSettings.llm.localBaseUrl]);

  useEffect(() => {
    if (historyPage !== clampedHistoryPage) {
      setHistoryPage(clampedHistoryPage);
    }
  }, [historyPage, clampedHistoryPage]);

  function updateProviderSettings(updater: (current: ProviderSettings) => ProviderSettings) {
    setProviderSettings((current) => updater(current));
  }

  function updateAsrCloudProvider(nextProvider: ASRCloudProvider) {
    updateProviderSettings((current) => ({
      ...current,
      asr: {
        ...current.asr,
        cloudProvider: nextProvider,
        cloudBaseUrl: getDefaultAsrCloudBaseUrl(nextProvider),
        cloudModel: getDefaultAsrCloudModel(nextProvider),
      },
    }));
  }

  function updateLlmCloudProvider(nextProvider: LLMCloudProvider) {
    updateProviderSettings((current) => ({
      ...current,
      llm: {
        ...current.llm,
        cloudProvider: nextProvider,
        cloudBaseUrl: getDefaultLlmCloudBaseUrl(nextProvider),
        cloudModel: getDefaultLlmCloudModel(nextProvider),
        cloudEmbeddingModel: getDefaultLlmEmbeddingModel(nextProvider),
      },
    }));
  }

  async function refreshDiagnostics(showFeedback = false) {
    setDiagnosticsRefreshing(true);
    try {
      const [audioStatus, diagnosticState] = await Promise.all([
        invoke<AudioInputStatus>("get_audio_input_status"),
        invoke<EnvironmentDiagnostics>("get_environment_diagnostics"),
      ]);
      setAudioInputLabel(
        `${audioStatus.deviceName} · ${audioStatus.sampleRate}Hz · ${audioStatus.channels}ch`,
      );
      setDiagnostics(diagnosticState);
      if (showFeedback) {
        setStatusMsg(ui.dashboard.refreshDiagnostics);
      }
    } catch (error) {
      setStatusMsg(normalizeStatusMessage(error instanceof Error ? error.message : String(error), locale));
    } finally {
      setDiagnosticsRefreshing(false);
    }
  }

  async function refreshLocalLlmModels(showFeedback = false) {
    setLocalLlmModelsRefreshing(true);
    try {
      const catalog = await invoke<LocalLLMModelCatalog>("get_local_llm_models");
      setLocalLlmModels(catalog);
      if (showFeedback) {
        setStatusMsg(
          catalog.reachable ? ui.dashboard.refreshModels : ui.dashboard.modelCatalogOffline,
        );
      }
    } catch (error) {
      setStatusMsg(normalizeStatusMessage(error instanceof Error ? error.message : String(error), locale));
    } finally {
      setLocalLlmModelsRefreshing(false);
    }
  }

  async function persistLocale(next: Locale) {
    setLocale(next);
    const nextSettings: ProviderSettings = {
      ...providerSettings,
      locale: next,
    };
    setProviderSettings(nextSettings);

    try {
      const envelope = await invoke<ProviderSettingsEnvelope>("update_provider_settings", {
        settings: nextSettings,
      });
      setProviderSettings(envelope.settings);
      setSettingsPath(envelope.settingsPath);
      await refreshDiagnostics();
      setStatusMsg(UI[next].statuses.saved);
    } catch (error) {
      setStatusMsg(
        normalizeStatusMessage(error instanceof Error ? error.message : String(error), next),
      );
    }
  }

  async function saveProviderSettings() {
    setSettingsSaving(true);
    try {
      const envelope = await invoke<ProviderSettingsEnvelope>("update_provider_settings", {
        settings: providerSettings,
      });
      setProviderSettings(envelope.settings);
      setSettingsPath(envelope.settingsPath);
      await refreshDiagnostics();
      setStatusMsg(ui.statuses.saved);
    } catch (error) {
      setStatusMsg(normalizeStatusMessage(error instanceof Error ? error.message : String(error), locale));
    } finally {
      setSettingsSaving(false);
    }
  }

  async function persistProviderSettings(nextSettings: ProviderSettings, successMessage?: string) {
    try {
      const envelope = await invoke<ProviderSettingsEnvelope>("update_provider_settings", {
        settings: nextSettings,
      });
      setProviderSettings(envelope.settings);
      setSettingsPath(envelope.settingsPath);
      await refreshDiagnostics();
      if (successMessage) {
        setStatusMsg(successMessage);
      }
    } catch (error) {
      setStatusMsg(normalizeStatusMessage(error instanceof Error ? error.message : String(error), locale));
    }
  }

  async function updateAsrLanguage(nextLanguage: string) {
    const nextSettings: ProviderSettings = {
      ...providerSettings,
      asr: {
        ...providerSettings.asr,
        language: nextLanguage,
      },
    };
    setProviderSettings(nextSettings);
    await persistProviderSettings(nextSettings, ui.statuses.saved);
  }

  async function downloadSelectedAsrModel() {
    setDownloadingAsr(true);
    try {
      const status = await invoke<LocalASRModelStatus>("download_local_asr_model", {
        request: { modelName: providerSettings.asr.localModel },
      });
      setLocalAsrStatus(status);
      setStatusMsg(ui.statuses.downloaded(status.modelName));
    } catch (error) {
      setStatusMsg(normalizeStatusMessage(error instanceof Error ? error.message : String(error), locale));
    } finally {
      setDownloadingAsr(false);
    }
  }

  const sidebarItems: Array<{ key: DashboardSection; label: string }> = [
    { key: "overview", label: ui.dashboard.overviewSection },
    { key: "general", label: ui.dashboard.generalSection },
    { key: "speech", label: ui.dashboard.speechSection },
    { key: "refine", label: ui.dashboard.refineSection },
    { key: "wake", label: ui.dashboard.wakeSection },
    { key: "history", label: ui.dashboard.historySection },
  ];

  if (!dashboardReady) {
    return (
      <div className="dashboard-app dashboard-loading-app">
        <div className="dashboard-loading-shell">
          <div className="dashboard-loading-kicker">Aura</div>
          <div className="dashboard-loading-title">
            {locale === "zh" ? "正在准备工作台" : "Preparing your workspace"}
          </div>
          <div className="dashboard-loading-copy">
            {locale === "zh"
              ? "正在加载设置、语音能力和最近记录。"
              : "Loading settings, voice capabilities, and recent history."}
          </div>
          <div className="dashboard-loading-row">
            <span className="dashboard-loading-dot" aria-hidden="true" />
            <span>{locale === "zh" ? "正在启动 Aura" : "Starting Aura"}</span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="dashboard-app">
      <div className="dashboard-shell">
        {statusMsg && <div className="dashboard-toast">{statusMsg}</div>}
        <header className="dashboard-header">
          <div>
            <div className="dashboard-kicker">Aura</div>
            <h1>{ui.dashboard.title}</h1>
            <p>{ui.dashboard.subtitle}</p>
          </div>
          <div className="dashboard-meta">
            <span>{ui.dashboard.hotkey}: Option + Shift + Space</span>
            <span>{audioInputLabel || ui.statuses.checkingInput}</span>
            <div className="language-switcher">
              <span className="language-switcher-icon" aria-hidden="true">◎</span>
              <div className="language-toggle" role="group" aria-label={ui.dashboard.language}>
                <button
                  type="button"
                  className={locale === "zh" ? "active" : ""}
                  onClick={() => void persistLocale("zh")}
                >
                  中文
                </button>
                <button
                  type="button"
                  className={locale === "en" ? "active" : ""}
                  onClick={() => void persistLocale("en")}
                >
                  EN
                </button>
              </div>
            </div>
            <div className="dashboard-header-actions">
              <div className="dashboard-header-actions-copy">
                <div className="dashboard-header-actions-title">{ui.dashboard.saveSettings}</div>
                <div className="dashboard-header-actions-note">{ui.dashboard.subtitle}</div>
              </div>
              <button
                type="button"
                className="save-button"
                disabled={settingsSaving}
                onClick={() => void saveProviderSettings()}
              >
                {settingsSaving ? ui.dashboard.saving : ui.dashboard.saveSettings}
              </button>
            </div>
          </div>
        </header>

        <main className="dashboard-layout">
          <aside className="dashboard-sidebar">
            {sidebarItems.map((item) => (
              <button
                key={item.key}
                type="button"
                className={`sidebar-item ${activeSection === item.key ? "active" : ""}`}
                onClick={() => setActiveSection(item.key)}
              >
                {item.label}
              </button>
            ))}
          </aside>

          <section className="dashboard-content">
          {activeSection === "overview" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.overviewSection}</div>
            <div className="overview-hero">
              <div>
                <h2>{ui.dashboard.overviewTitle}</h2>
                <p>{ui.dashboard.overviewDescription}</p>
              </div>
              <div className="overview-pill">
                <span className="overview-pill-dot" />
                <span>{ui.dashboard.capsuleReady}</span>
              </div>
            </div>

            <div className="overview-callout">
              <div className="overview-callout-label">{ui.dashboard.hotkey}</div>
              <div className="overview-callout-value">Option + Shift + Space</div>
              <div className="overview-callout-note">{ui.dashboard.capsuleReadyNote}</div>
            </div>

            <div className="overview-grid">
              <div className="overview-card">
                <div className="overview-card-label">{ui.dashboard.activeInput}</div>
                <div className="overview-card-value">{audioInputLabel || ui.statuses.checkingInput}</div>
              </div>
              <div className="overview-card">
                <div className="overview-card-label">{ui.dashboard.speechProvider}</div>
                <div className="overview-card-value">
                  {providerSettings.asr.provider === "local"
                    ? `${ui.dashboard.local} · ${providerSettings.asr.localModel}`
                    : `${ui.dashboard.cloud} · ${providerSettings.asr.cloudModel}`}
                </div>
              </div>
              <div className="overview-card">
                <div className="overview-card-label">{ui.dashboard.refineProvider}</div>
                <div className="overview-card-value">
                  {providerSettings.llm.provider === "local"
                    ? `${ui.dashboard.local} · ${providerSettings.llm.localModel}`
                    : `${ui.dashboard.cloud} · ${providerSettings.llm.cloudModel}`}
                </div>
              </div>
              <div className="overview-card">
                <div className="overview-card-label">{ui.dashboard.wakeWordCard}</div>
                <div className="overview-card-value">
                  {providerSettings.wakeWordEnabled ? ui.dashboard.active : ui.dashboard.inactive}
                </div>
              </div>
            </div>

            {diagnostics && (
              <div className="overview-diagnostics">
                <div className="overview-diagnostics-header">
                  <div className="overview-history-title">{ui.dashboard.diagnostics}</div>
                  <button
                    type="button"
                    className="secondary-button"
                    disabled={diagnosticsRefreshing}
                    onClick={() => void refreshDiagnostics(true)}
                  >
                    {diagnosticsRefreshing
                      ? ui.dashboard.refreshingDiagnostics
                      : ui.dashboard.refreshDiagnostics}
                  </button>
                </div>
                <div className="overview-diagnostics-grid">
                  {[diagnostics.speech, diagnostics.refine, diagnostics.delivery].map((item) => (
                    <div
                      key={item.title}
                      className={`overview-diagnostic-card ${item.ready ? "ready" : "attention"}`}
                    >
                      <div className="overview-diagnostic-top">
                        <div className="overview-card-label">{item.title}</div>
                        <div className={`overview-diagnostic-badge ${item.ready ? "ready" : "attention"}`}>
                          {item.ready ? ui.dashboard.ready : ui.dashboard.needsAttention}
                        </div>
                      </div>
                      <div className="overview-diagnostic-detail">{item.detail}</div>
                      {!item.ready && item.title === "Auto-paste" && (
                        <button
                          type="button"
                          className="overview-diagnostic-link"
                          onClick={() => void invoke("open_accessibility_settings")}
                        >
                          {ui.dashboard.openAccessibility}
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className="overview-history">
              <div className="overview-history-title">{ui.dashboard.recentActivity}</div>
              {historyItems.length === 0 ? (
                <div className="overview-history-empty">{ui.dashboard.noRecentActivity}</div>
              ) : (
                <div className="overview-history-card">
                  <div className="overview-history-label">{ui.dashboard.lastResult}</div>
                  <div className="overview-history-text">{historyItems[0].refined}</div>
                </div>
              )}
            </div>
          </section>
          )}

          {activeSection === "general" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.generalTitle}</div>
            <div className="panel-note">{ui.dashboard.generalDescription}</div>
            <div className="field-grid compact-top">
              <label>
                <span>{ui.dashboard.activeInput}</span>
                <input value={audioInputLabel || ui.statuses.checkingInput} readOnly />
              </label>
            </div>
            {settingsPath && <div className="panel-note">{ui.dashboard.settingsFile}: {settingsPath}</div>}
          </section>
          )}

          {activeSection === "speech" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.speechSection}</div>
            <div className="segmented">
              {(["local", "cloud"] as ProviderMode[]).map((kind) => (
                <button
                  key={kind}
                  type="button"
                  className={providerSettings.asr.provider === kind ? "active" : ""}
                  onClick={() =>
                    updateProviderSettings((current) => ({
                      ...current,
                      asr: { ...current.asr, provider: kind },
                    }))
                  }
                >
                  {kind === "local" ? ui.dashboard.local : ui.dashboard.cloud}
                </button>
              ))}
            </div>
            <div className="field-grid">
              {providerSettings.asr.provider === "local" ? (
                <>
                  <label>
                    <span>{ui.dashboard.speechLanguage}</span>
                    <select
                      value={providerSettings.asr.language}
                      onChange={(event) => void updateAsrLanguage(event.target.value)}
                    >
                      <option value="auto">{ui.dashboard.speechLanguageAuto}</option>
                      <option value="zh">{ui.dashboard.speechLanguageZh}</option>
                      <option value="en">{ui.dashboard.speechLanguageEn}</option>
                    </select>
                  </label>
                  <label>
                    <span>{ui.dashboard.localModel}</span>
                    <select
                      value={providerSettings.asr.localModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          asr: { ...current.asr, localModel: event.target.value },
                        }))
                      }
                    >
                      {localAsrChoices.map((choice) => (
                        <option key={choice.value} value={choice.value}>
                          {choice.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="model-recommendation">
                    <span className="model-recommendation-badge">
                      {selectedLocalAsr.value === "whisper-base"
                        ? ui.dashboard.recommended
                        : ui.dashboard.note}
                    </span>
                    <span>{selectedLocalAsr.hint}</span>
                  </div>
                  <div className="model-download-card">
                    <div>
                      <div className="model-download-title">
                        {localAsrStatus?.downloaded
                          ? ui.dashboard.downloadedLocal
                          : ui.dashboard.notDownloaded}
                      </div>
                      <div className="model-download-meta">
                        {localAsrStatus?.downloaded
                          ? `${localAsrStatus.sizeMb.toFixed(1)} MB · ${localAsrStatus.path}`
                          : ui.dashboard.estimatedDownload(localAsrStatus?.suggestedDownloadMb ?? 142)}
                      </div>
                      <div className="model-download-subtle">{ui.dashboard.installerHint}</div>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={downloadingAsr}
                      onClick={() => void downloadSelectedAsrModel()}
                    >
                      {downloadingAsr
                        ? ui.dashboard.preparingDownload
                        : localAsrStatus?.downloaded
                          ? ui.dashboard.redownload
                          : ui.dashboard.downloadModel}
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <label>
                    <span>{ui.dashboard.provider}</span>
                    <select
                      value={providerSettings.asr.cloudProvider}
                      onChange={(event) => updateAsrCloudProvider(event.target.value as ASRCloudProvider)}
                    >
                      <option value="openai">{ui.dashboard.providerOpenAI}</option>
                      <option value="groq">{ui.dashboard.providerGroq}</option>
                      <option value="deepgram">{ui.dashboard.providerDeepgram}</option>
                      <option value="assemblyai">{ui.dashboard.providerAssemblyAI}</option>
                      <option value="azure">{ui.dashboard.providerAzure}</option>
                      <option value="google">{ui.dashboard.providerGoogle}</option>
                      <option value="custom">{ui.dashboard.providerCustom}</option>
                    </select>
                  </label>
                  <div className="panel-note">{ui.dashboard.asrProviderHint}</div>
                  <label>
                    <span>{ui.dashboard.speechLanguage}</span>
                    <select
                      value={providerSettings.asr.language}
                      onChange={(event) => void updateAsrLanguage(event.target.value)}
                    >
                      <option value="auto">{ui.dashboard.speechLanguageAuto}</option>
                      <option value="zh">{ui.dashboard.speechLanguageZh}</option>
                      <option value="en">{ui.dashboard.speechLanguageEn}</option>
                    </select>
                  </label>
                  <label>
                    <span>{ui.dashboard.apiBaseUrl}</span>
                    <input
                      value={providerSettings.asr.cloudBaseUrl}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          asr: { ...current.asr, cloudBaseUrl: event.target.value },
                        }))
                      }
                      placeholder="https://api.openai.com/v1"
                    />
                  </label>
                  <label>
                    <span>{ui.dashboard.recommendedModels}</span>
                    <select
                      value={providerSettings.asr.cloudModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          asr: { ...current.asr, cloudModel: event.target.value },
                        }))
                      }
                    >
                      {ASR_CLOUD_MODEL_OPTIONS[providerSettings.asr.cloudProvider].map((choice) => (
                        <option key={choice.value} value={choice.value}>
                          {choice.label}
                        </option>
                      ))}
                      <option value={providerSettings.asr.cloudModel}>{ui.dashboard.customModel}</option>
                    </select>
                  </label>
                  <label>
                    <span>{ui.dashboard.model}</span>
                    <input
                      value={providerSettings.asr.cloudModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          asr: { ...current.asr, cloudModel: event.target.value },
                        }))
                      }
                      placeholder="gpt-4o-mini-transcribe"
                    />
                  </label>
                  <label>
                    <span>{ui.dashboard.apiKey}</span>
                    <input
                      type="password"
                      value={providerSettings.asr.cloudApiKey}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          asr: { ...current.asr, cloudApiKey: event.target.value },
                        }))
                      }
                      placeholder="sk-..."
                    />
                  </label>
                </>
              )}
            </div>
          </section>
          )}

          {activeSection === "refine" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.refineSection}</div>
            <div className="segmented">
              {(["local", "cloud"] as ProviderMode[]).map((kind) => (
                <button
                  key={kind}
                  type="button"
                  className={providerSettings.llm.provider === kind ? "active" : ""}
                  onClick={() =>
                    updateProviderSettings((current) => ({
                      ...current,
                      llm: { ...current.llm, provider: kind },
                    }))
                  }
                >
                  {kind === "local" ? ui.dashboard.local : ui.dashboard.cloud}
                </button>
              ))}
            </div>
            <div className="field-grid">
              {providerSettings.llm.provider === "local" ? (
                <>
                  <label>
                    <span>{ui.dashboard.localModel}</span>
                    <select
                      value={providerSettings.llm.localModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, localModel: event.target.value },
                        }))
                      }
                    >
                      {localLlmChoices.map((model) => (
                        <option key={model} value={model}>
                          {model}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="model-recommendation">
                    <span className="model-recommendation-badge">
                      {localLlmModels?.reachable ? ui.dashboard.local : ui.dashboard.note}
                    </span>
                    <span>
                      {localLlmModels?.reachable
                        ? `${localLlmChoices.length} ${locale === "zh" ? "个本地模型可选" : "local models available"}`
                        : ui.dashboard.modelCatalogOffline}
                    </span>
                  </div>
                  <div className="model-download-card">
                    <div>
                      <div className="model-download-title">{ui.dashboard.localModel}</div>
                      <div className="model-download-meta">{providerSettings.llm.localModel}</div>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={localLlmModelsRefreshing}
                      onClick={() => void refreshLocalLlmModels(true)}
                    >
                      {localLlmModelsRefreshing
                        ? ui.dashboard.refreshingDiagnostics
                        : ui.dashboard.refreshModels}
                    </button>
                  </div>
                  <label>
                    <span>{ui.dashboard.ollamaUrl}</span>
                    <input
                      value={providerSettings.llm.localBaseUrl}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, localBaseUrl: event.target.value },
                        }))
                      }
                      placeholder="http://localhost:11434"
                    />
                  </label>
                </>
              ) : (
                <>
                  <label>
                    <span>{ui.dashboard.provider}</span>
                    <select
                      value={providerSettings.llm.cloudProvider}
                      onChange={(event) => updateLlmCloudProvider(event.target.value as LLMCloudProvider)}
                    >
                      <option value="openai">{ui.dashboard.providerOpenAI}</option>
                      <option value="anthropic">{ui.dashboard.providerAnthropic}</option>
                      <option value="gemini">{ui.dashboard.providerGemini}</option>
                      <option value="deepseek">{ui.dashboard.providerDeepSeek}</option>
                      <option value="qwen">{ui.dashboard.providerQwen}</option>
                      <option value="glm">{ui.dashboard.providerGlm}</option>
                      <option value="kimi">{ui.dashboard.providerKimi}</option>
                      <option value="minimax">{ui.dashboard.providerMinimax}</option>
                      <option value="openrouter">{ui.dashboard.providerOpenRouter}</option>
                      <option value="custom">{ui.dashboard.providerCustom}</option>
                    </select>
                  </label>
                  <div className="panel-note">{ui.dashboard.llmProviderHint}</div>
                  <label>
                    <span>{ui.dashboard.apiBaseUrl}</span>
                    <input
                      value={providerSettings.llm.cloudBaseUrl}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, cloudBaseUrl: event.target.value },
                        }))
                      }
                      placeholder="https://api.openai.com/v1"
                    />
                  </label>
                  <label>
                    <span>{ui.dashboard.recommendedModels}</span>
                    <select
                      value={providerSettings.llm.cloudModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, cloudModel: event.target.value },
                        }))
                      }
                    >
                      {LLM_CLOUD_MODEL_OPTIONS[providerSettings.llm.cloudProvider].map((choice) => (
                        <option key={choice.value} value={choice.value}>
                          {choice.label}
                        </option>
                      ))}
                      <option value={providerSettings.llm.cloudModel}>{ui.dashboard.customModel}</option>
                    </select>
                  </label>
                  <label>
                    <span>{ui.dashboard.model}</span>
                    <input
                      value={providerSettings.llm.cloudModel}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, cloudModel: event.target.value },
                        }))
                      }
                      placeholder="gpt-4.1-mini"
                    />
                  </label>
                  <label>
                    <span>{ui.dashboard.apiKey}</span>
                    <input
                      type="password"
                      value={providerSettings.llm.cloudApiKey}
                      onChange={(event) =>
                        updateProviderSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, cloudApiKey: event.target.value },
                        }))
                      }
                      placeholder="sk-..."
                    />
                  </label>
                </>
              )}
            </div>
          </section>
          )}

          {activeSection === "wake" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.wakeTitle}</div>
            <div className="panel-note">{ui.dashboard.wakeDescription}</div>
            <div className="field-grid compact-top">
              <label className="toggle-row">
                <span>{ui.dashboard.wakeEnabled}</span>
                <input
                  type="checkbox"
                  checked={providerSettings.wakeWordEnabled}
                  onChange={(event) =>
                    updateProviderSettings((current) => ({
                      ...current,
                      wakeWordEnabled: event.target.checked,
                    }))
                  }
                />
              </label>
              <label>
                <span>{ui.dashboard.wakePhrase}</span>
                <input
                  value={providerSettings.wakeWordPhrase}
                  onChange={(event) =>
                    updateProviderSettings((current) => ({
                      ...current,
                      wakeWordPhrase: event.target.value,
                    }))
                  }
                  placeholder="Aura"
                />
              </label>
            </div>
            <div className="panel-note">{ui.dashboard.wakeHint}</div>
          </section>
          )}

          {activeSection === "history" && (
          <section className="panel">
            <div className="panel-title">{ui.dashboard.historySection}</div>
            {historyItems.length === 0 ? (
              <div className="history-empty">{ui.dashboard.emptyHistory}</div>
            ) : (
              <>
                <div className="history-list">
                  {pagedHistoryItems.map((item) => (
                  <div className="history-item" key={item.id}>
                    <div className="history-meta">
                      <span>{item.delivered ? ui.dashboard.delivered : ui.dashboard.copied}</span>
                      <span>
                        {new Date(item.timestamp).toLocaleString(locale === "zh" ? "zh-CN" : "en-US", {
                          month: "2-digit",
                          day: "2-digit",
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                    <div className="history-transcript">{item.transcript}</div>
                    <div className="history-text">{item.refined}</div>
                  </div>
                ))}
                </div>
                {totalHistoryPages > 1 && (
                  <div className="history-pagination">
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={clampedHistoryPage <= 1}
                      onClick={() => setHistoryPage((page) => Math.max(1, page - 1))}
                    >
                      {ui.dashboard.prevPage}
                    </button>
                    <div className="history-page-info">
                      {ui.dashboard.historyPagination(clampedHistoryPage, totalHistoryPages)}
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={clampedHistoryPage >= totalHistoryPages}
                      onClick={() =>
                        setHistoryPage((page) => Math.min(totalHistoryPages, page + 1))
                      }
                    >
                      {ui.dashboard.nextPage}
                    </button>
                  </div>
                )}
              </>
            )}
          </section>
          )}
          </section>
        </main>
      </div>
    </div>
  );
}

function App() {
  const isCapsuleWindow = window.location.hash === "#capsule";
  return isCapsuleWindow ? <CapsuleApp /> : <DashboardApp />;
}

export default App;
