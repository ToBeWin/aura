import { useRef, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./ContextManager.css";

interface UserContext {
  user_id: string;
  name_mappings: Record<string, string>;
  location_preferences: Record<string, string>;
  terminology: Record<string, string>;
  forbidden_words: string[];
  default_tone: string;
  default_format: string | null;
  created_at: string;
  updated_at: string;
}

interface ContextManagerProps {
  userId: string;
  onClose: () => void;
}

export function ContextManager({ userId, onClose }: ContextManagerProps) {
  const [context, setContext] = useState<UserContext>({
    user_id: userId,
    name_mappings: {},
    location_preferences: {},
    terminology: {},
    forbidden_words: [],
    default_tone: "professional",
    default_format: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  });
  const [lastSavedContext, setLastSavedContext] = useState<string>("");
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [feedback, setFeedback] = useState("");
  const [newNameKey, setNewNameKey] = useState("");
  const [newNameValue, setNewNameValue] = useState("");
  const [newLocation, setNewLocation] = useState("");
  const [newLocationValue, setNewLocationValue] = useState("");
  const [newTerm, setNewTerm] = useState("");
  const [newTermValue, setNewTermValue] = useState("");
  const [newForbiddenWord, setNewForbiddenWord] = useState("");
  const [showMoreActions, setShowMoreActions] = useState(false);
  const [activeEdit, setActiveEdit] = useState<string>("");
  const nameKeyRef = useRef<HTMLInputElement | null>(null);
  const nameValueRef = useRef<HTMLInputElement | null>(null);
  const locationKeyRef = useRef<HTMLInputElement | null>(null);
  const locationValueRef = useRef<HTMLInputElement | null>(null);
  const termKeyRef = useRef<HTMLInputElement | null>(null);
  const termValueRef = useRef<HTMLInputElement | null>(null);
  const avoidRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    loadContext();
  }, [userId]);

  async function loadContext() {
    try {
      const ctx = await invoke<UserContext>("get_user_context", { userId });
      setContext(ctx);
      setLastSavedContext(JSON.stringify(ctx));
      setSaveState("idle");
      setFeedback("");
    } catch (error) {
      console.error("Failed to load context:", error);
      setSaveState("error");
      setFeedback("Aura Memory could not load this profile yet.");
    }
  }

  async function saveContext() {
    setSaveState("saving");
    setFeedback("Saving Aura Memory…");
    try {
      await invoke("update_user_context", { context });
      const snapshot = JSON.stringify(context);
      setLastSavedContext(snapshot);
      setSaveState("saved");
      setFeedback("Aura Memory updated. New drafts will use these rules.");
    } catch (error) {
      console.error("Failed to save context:", error);
      setSaveState("error");
      setFeedback("Save failed. Your edits are still here.");
    }
  }

  function addNameMapping() {
    if (newNameKey && newNameValue) {
      setContext({
        ...context,
        name_mappings: { ...context.name_mappings, [newNameKey]: newNameValue },
      });
      setNewNameKey("");
      setNewNameValue("");
      setActiveEdit("");
    }
  }

  function removeNameMapping(key: string) {
    const { [key]: _, ...rest } = context.name_mappings;
    setContext({ ...context, name_mappings: rest });
  }

  function addLocation() {
    if (newLocation && newLocationValue) {
      setContext({
        ...context,
        location_preferences: { ...context.location_preferences, [newLocation]: newLocationValue },
      });
      setNewLocation("");
      setNewLocationValue("");
      setActiveEdit("");
    }
  }

  function removeLocation(key: string) {
    const { [key]: _, ...rest } = context.location_preferences;
    setContext({ ...context, location_preferences: rest });
  }

  function addTerminology() {
    if (newTerm && newTermValue) {
      setContext({
        ...context,
        terminology: { ...context.terminology, [newTerm]: newTermValue },
      });
      setNewTerm("");
      setNewTermValue("");
      setActiveEdit("");
    }
  }

  function removeTerminology(key: string) {
    const { [key]: _, ...rest } = context.terminology;
    setContext({ ...context, terminology: rest });
  }

  function addForbiddenWord() {
    if (newForbiddenWord && !context.forbidden_words.includes(newForbiddenWord)) {
      setContext({
        ...context,
        forbidden_words: [...context.forbidden_words, newForbiddenWord],
      });
      setNewForbiddenWord("");
      setActiveEdit("");
    }
  }

  function removeForbiddenWord(word: string) {
    setContext({
      ...context,
      forbidden_words: context.forbidden_words.filter(w => w !== word),
    });
  }

  async function exportContext() {
    try {
      const json = JSON.stringify(context, null, 2);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `aura-context-${userId}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error("Failed to export context:", error);
    }
  }

  async function importContext(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      const imported = JSON.parse(text);
      setContext(imported);
      setSaveState("idle");
      setFeedback("Imported memory file. Review it, then save to apply.");
    } catch (error) {
      console.error("Failed to import context:", error);
      setSaveState("error");
      setFeedback("Import failed. Please use a valid Aura memory JSON file.");
    } finally {
      event.target.value = "";
    }
  }

  const dirty = JSON.stringify(context) !== lastSavedContext;
  const latestName = Object.entries(context.name_mappings)[0];
  const latestPlace = Object.entries(context.location_preferences)[0];
  const latestTerm = Object.entries(context.terminology)[0];
  const latestAvoid = context.forbidden_words[0];
  const summaryCards = [
    {
      label: "Names",
      value: Object.keys(context.name_mappings).length,
      hint: "Alias corrections",
      preview: latestName ? `${latestName[0]} -> ${latestName[1]}` : "Try: 苏 -> 苏总",
    },
    {
      label: "Places",
      value: Object.keys(context.location_preferences).length,
      hint: "Location shortcuts",
      preview: latestPlace ? `${latestPlace[0]} -> ${latestPlace[1]}` : "Try: 家里 -> 上海静安",
    },
    {
      label: "Terms",
      value: Object.keys(context.terminology).length,
      hint: "Preferred wording",
      preview: latestTerm ? `${latestTerm[0]} -> ${latestTerm[1]}` : "Try: AI -> 人工智能",
    },
    {
      label: "Avoid",
      value: context.forbidden_words.length,
      hint: "Blocked phrases",
      preview: latestAvoid ? `Avoid: ${latestAvoid}` : "Try: 那个",
    },
  ];
  const nextBehavior = dirty
    ? "Save these edits and Aura will use them on your next capture."
    : latestPlace
      ? `Next capture will expand "${latestPlace[0]}" to "${latestPlace[1]}".`
      : latestName
        ? `Next capture will write "${latestName[1]}" when you say "${latestName[0]}".`
        : context.forbidden_words.length > 0
          ? `Aura will avoid ${context.forbidden_words.length} blocked phrase${context.forbidden_words.length > 1 ? "s" : ""} in new drafts.`
          : "Add one memory rule and Aura will start shaping drafts around it.";

  function applyNameExample(from: string, to: string) {
    setNewNameKey(from);
    setNewNameValue(to);
    setActiveEdit("names");
    setTimeout(() => nameValueRef.current?.focus(), 0);
  }

  function applyPlaceExample(from: string, to: string) {
    setNewLocation(from);
    setNewLocationValue(to);
    setActiveEdit("places");
    setTimeout(() => locationValueRef.current?.focus(), 0);
  }

  function applyTermExample(from: string, to: string) {
    setNewTerm(from);
    setNewTermValue(to);
    setActiveEdit("terms");
    setTimeout(() => termValueRef.current?.focus(), 0);
  }

  function applyAvoidExample(term: string) {
    setNewForbiddenWord(term);
    setActiveEdit("avoid");
    setTimeout(() => avoidRef.current?.focus(), 0);
  }

  return (
    <div className="context-modal">
      <div className="context-content">
        <div className="context-header">
          <div className="context-heading">
            <span className="context-kicker">Aura Memory</span>
            <h2>Teach Aura the names, places, and words that should feel automatic.</h2>
            <p>
              These rules are applied quietly while Aura shapes drafts, so you do less fixing after each capture.
            </p>
          </div>
          <button onClick={onClose} className="close-btn">✕</button>
        </div>

        <div className="context-body">
          <section className="memory-overview">
            <div className="memory-grid">
              {summaryCards.map((card) => (
                <article key={card.label} className="memory-card">
                  <span className="memory-card-label">{card.label}</span>
                  <strong>{card.value}</strong>
                  <span className="memory-card-hint">{card.hint}</span>
                  <span className="memory-card-preview">{card.preview}</span>
                </article>
              ))}
            </div>
            <div className={`memory-status ${saveState} ${dirty ? "dirty" : ""}`}>
              <span className="memory-status-dot" />
              <span>
                {feedback || nextBehavior}
              </span>
            </div>
          </section>

          <section className="context-section">
            <h3>Names</h3>
            <p className="section-hint">Map shorthand mentions to the exact person Aura should use.</p>
            <div className="example-row">
              <button className="example-chip" onClick={() => applyNameExample("苏", "苏总")} type="button">苏 → 苏总</button>
              <button className="example-chip" onClick={() => applyNameExample("小李", "李经理")} type="button">小李 → 李经理</button>
              <button className="example-chip" onClick={() => applyNameExample("Sherry", "Sherry Wang")} type="button">Sherry → Sherry Wang</button>
            </div>
            <div className="mapping-list">
              {Object.entries(context.name_mappings).map(([key, value]) => (
                <div
                  key={key}
                  className={`mapping-item ${activeEdit === `name:${key}` ? "active" : ""}`}
                  onClick={() => {
                    setNewNameKey(key);
                    setNewNameValue(value);
                    setActiveEdit(`name:${key}`);
                    setTimeout(() => nameValueRef.current?.focus(), 0);
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <span className="mapping-key">{key}</span>
                  <span className="mapping-arrow">→</span>
                  <span className="mapping-value">{value}</span>
                  <button
                    onClick={(event) => {
                      event.stopPropagation();
                      removeNameMapping(key);
                    }}
                    className="remove-btn"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
            <div className="add-mapping">
              <input
                type="text"
                placeholder="What you say"
                value={newNameKey}
                onChange={(e) => setNewNameKey(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newNameKey && newNameValue) {
                    addNameMapping();
                  }
                }}
                className="mapping-input"
                ref={nameKeyRef}
              />
              <span>→</span>
              <input
                type="text"
                placeholder="What Aura should write"
                value={newNameValue}
                onChange={(e) => setNewNameValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newNameKey && newNameValue) {
                    addNameMapping();
                  }
                }}
                className="mapping-input"
                ref={nameValueRef}
              />
              <button onClick={addNameMapping} className="add-btn">Add</button>
            </div>
          </section>

          <section className="context-section">
            <h3>Places</h3>
            <p className="section-hint">Expand local shortcuts into the place names you actually mean.</p>
            <div className="example-row">
              <button className="example-chip" onClick={() => applyPlaceExample("家里", "上海静安")} type="button">家里 → 上海静安</button>
              <button className="example-chip" onClick={() => applyPlaceExample("办公室", "浦东张江办公室")} type="button">办公室 → 浦东张江办公室</button>
              <button className="example-chip" onClick={() => applyPlaceExample("机场", "虹桥机场 T2")} type="button">机场 → 虹桥机场 T2</button>
            </div>
            <div className="mapping-list">
              {Object.entries(context.location_preferences).map(([key, value]) => (
                <div
                  key={key}
                  className={`mapping-item ${activeEdit === `place:${key}` ? "active" : ""}`}
                  onClick={() => {
                    setNewLocation(key);
                    setNewLocationValue(value);
                    setActiveEdit(`place:${key}`);
                    setTimeout(() => locationValueRef.current?.focus(), 0);
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <span className="mapping-key">{key}</span>
                  <span className="mapping-arrow">→</span>
                  <span className="mapping-value">{value}</span>
                  <button
                    onClick={(event) => {
                      event.stopPropagation();
                      removeLocation(key);
                    }}
                    className="remove-btn"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
            <div className="add-mapping">
              <input
                type="text"
                placeholder="Shortcut"
                value={newLocation}
                onChange={(e) => setNewLocation(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newLocation && newLocationValue) {
                    addLocation();
                  }
                }}
                className="mapping-input"
                ref={locationKeyRef}
              />
              <span>→</span>
              <input
                type="text"
                placeholder="Expanded place"
                value={newLocationValue}
                onChange={(e) => setNewLocationValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newLocation && newLocationValue) {
                    addLocation();
                  }
                }}
                className="mapping-input"
                ref={locationValueRef}
              />
              <button onClick={addLocation} className="add-btn">Add</button>
            </div>
          </section>

          <section className="context-section">
            <h3>Terms</h3>
            <p className="section-hint">Keep product names, jargon, and phrases consistent across drafts.</p>
            <div className="example-row">
              <button className="example-chip" onClick={() => applyTermExample("AI", "人工智能")} type="button">AI → 人工智能</button>
              <button className="example-chip" onClick={() => applyTermExample("GMV", "成交总额")} type="button">GMV → 成交总额</button>
              <button className="example-chip" onClick={() => applyTermExample("agent", "智能体")} type="button">agent → 智能体</button>
            </div>
            <div className="mapping-list">
              {Object.entries(context.terminology).map(([key, value]) => (
                <div
                  key={key}
                  className={`mapping-item ${activeEdit === `term:${key}` ? "active" : ""}`}
                  onClick={() => {
                    setNewTerm(key);
                    setNewTermValue(value);
                    setActiveEdit(`term:${key}`);
                    setTimeout(() => termValueRef.current?.focus(), 0);
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <span className="mapping-key">{key}</span>
                  <span className="mapping-arrow">→</span>
                  <span className="mapping-value">{value}</span>
                  <button
                    onClick={(event) => {
                      event.stopPropagation();
                      removeTerminology(key);
                    }}
                    className="remove-btn"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
            <div className="add-mapping">
              <input
                type="text"
                placeholder="Raw phrase"
                value={newTerm}
                onChange={(e) => setNewTerm(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newTerm && newTermValue) {
                    addTerminology();
                  }
                }}
                className="mapping-input"
                ref={termKeyRef}
              />
              <span>→</span>
              <input
                type="text"
                placeholder="Preferred wording"
                value={newTermValue}
                onChange={(e) => setNewTermValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newTerm && newTermValue) {
                    addTerminology();
                  }
                }}
                className="mapping-input"
                ref={termValueRef}
              />
              <button onClick={addTerminology} className="add-btn">Add</button>
            </div>
          </section>

          <section className="context-section">
            <h3>Avoid</h3>
            <p className="section-hint">Tell Aura which words or phrases should never show up in final copy.</p>
            <div className="example-row">
              <button className="example-chip" onClick={() => applyAvoidExample("那个")} type="button">Avoid “那个”</button>
              <button className="example-chip" onClick={() => applyAvoidExample("尽快处理")} type="button">Avoid “尽快处理”</button>
              <button className="example-chip" onClick={() => applyAvoidExample("差不多")} type="button">Avoid “差不多”</button>
            </div>
            <div className="tag-list">
              {context.forbidden_words.map((word) => (
                <div
                  key={word}
                  className={`tag-item ${activeEdit === `avoid:${word}` ? "active" : ""}`}
                  onClick={() => {
                    setNewForbiddenWord(word);
                    setActiveEdit(`avoid:${word}`);
                    setTimeout(() => avoidRef.current?.focus(), 0);
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <span>{word}</span>
                  <button
                    onClick={(event) => {
                      event.stopPropagation();
                      removeForbiddenWord(word);
                    }}
                    className="tag-remove"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
            <div className="add-tag">
              <input
                type="text"
                placeholder="Word or phrase to block"
                value={newForbiddenWord}
                onChange={(e) => setNewForbiddenWord(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addForbiddenWord()}
                className="tag-input"
                ref={avoidRef}
              />
              <button onClick={addForbiddenWord} className="add-btn">Add</button>
            </div>
          </section>
        </div>

        <div className="context-footer">
          <div className="footer-actions footer-actions-start">
            <div className={`more-actions ${showMoreActions ? "open" : ""}`}>
              <button
                className="more-btn"
                onClick={() => setShowMoreActions((current) => !current)}
                type="button"
              >
                More
              </button>
              {showMoreActions && (
                <div className="more-menu">
                  <label className="import-btn">
                    <input
                      type="file"
                      accept=".json"
                      onChange={importContext}
                      style={{ display: "none" }}
                    />
                    Import JSON
                  </label>
                  <button onClick={exportContext} className="export-btn" type="button">
                    Export JSON
                  </button>
                </div>
              )}
            </div>
          </div>
          <div className="footer-actions">
            <button onClick={onClose} className="cancel-btn">Cancel</button>
            <button onClick={saveContext} className="save-btn" disabled={!dirty || saveState === "saving"}>
              {saveState === "saving" ? "Saving…" : dirty ? "Save Memory" : "Saved"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
