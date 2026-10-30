<script lang="ts">
  import Menu from "../Menu.svelte";
  import Select from "$shared/ui/Select";
</script>

<!-- Stands in for a modal sheet: fixed over everything, at the sheet's layer. -->
<div
  role="dialog"
  aria-label="Sheet"
  style="position: fixed; inset: 0; z-index: 81; background: white; padding: 40px"
>
  <Menu label="Actions" entries={[{ kind: "item", id: "one", label: "One" }]} onselect={() => {}}>
    {#snippet trigger()}Actions{/snippet}
  </Menu>
  <Select
    label="Choice"
    value="one"
    options={[
      { value: "one", label: "One" },
      { value: "two", label: "Two" },
    ]}
  />
</div>
