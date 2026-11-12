<script lang="ts">
  type IconName =
    | "plus"
    | "send"
    | "stop"
    | "settings"
    | "close"
    | "pencil"
    | "trash"
    | "panel"
    | "book"
    | "message"
    | "user"
    | "alert"
    | "check"
    | "refresh";

  let {
    name,
    size = 16,
    strokeWidth = 1.6,
  }: { name: IconName; size?: number; strokeWidth?: number } = $props();

  const paths: Record<IconName, string> = {
    plus: "M12 5v14M5 12h14",
    send: "M12 19V5M5 12l7-7 7 7",
    stop: "M7 7h10v10H7z",
    settings:
      "M4 7h9M17 7h3M4 17h3M11 17h9M15 5a2 2 0 1 0 0 4 2 2 0 1 0 0-4M9 15a2 2 0 1 0 0 4 2 2 0 1 0 0-4",
    close: "M6 6l12 12M18 6L6 18",
    pencil: "M4 20l4.5-1L19 8.5 15.5 5 5 15.5 4 20Z",
    trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
    panel: "M4 5h16v14H4zM15 5v14",
    book: "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5v-15ZM4 20.5V21",
    message: "M20 11.5a8 8 0 0 1-8 8H7l-4 2 1.2-4.2A8 8 0 1 1 20 11.5Z",
    user: "M20 21a8 8 0 0 0-16 0M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z",
    alert: "M12 8v5M12 17h.01M10.3 3.8 2.6 19a1.8 1.8 0 0 0 1.6 2.6h15.6A1.8 1.8 0 0 0 21.4 19L13.7 3.8a1.9 1.9 0 0 0-3.4 0Z",
    check: "m5 12 4 4L19 6",
    refresh: "M20 6v5h-5M4 18v-5h5M6.5 9A7 7 0 0 1 18 7.5L20 11M17.5 15A7 7 0 0 1 6 16.5L4 13",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={name === "stop" ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={name === "stop" ? 0 : strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
</svg>
