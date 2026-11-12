<script lang="ts">
  let {
    name,
    accent = "#7c7468",
    size = 28,
  }: { name: string; accent?: string; size?: number } = $props();

  const initial = $derived(name.trim().slice(0, 1) || "？");
</script>

<span
  class="avatar"
  style={`--size:${size}px; --accent:${accent}`}
  aria-hidden="true">{initial}</span
>

<style>
  .avatar {
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--accent) 16%, #fffdf8);
    color: color-mix(in srgb, var(--accent) 78%, #2f2b26);
    border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
    font-size: calc(var(--size) * 0.4);
    font-weight: 550;
    line-height: 1;
    flex: none;
    user-select: none;
  }
</style>
