<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import PreferenceSelect from "../PreferenceSelect.svelte";
  import PreferenceSwitch from "../PreferenceSwitch.svelte";
</script>

<SettingsGroup title={m.tabs_group_opening()}
  ><PreferenceSelect id="tabs.new-position" preference="tabs.new-position" /><PreferenceSelect
    id="tabs.external"
    preference="tabs.external-position"
  /><PreferenceSwitch id="tabs.duplicates" preference="tabs.switch-to-open" /><PreferenceSwitch
    id="tabs.automatic-pip"
    preference="tabs.automatic-pip"
  /></SettingsGroup
>
<SettingsGroup title={m.tabs_group_closing()}
  ><PreferenceSelect id="tabs.close" preference="tabs.after-close" /><PreferenceSelect
    id="tabs.close-button"
    preference="tabs.close-button"
  /></SettingsGroup
>
<SettingsGroup title={m.panels_group()}
  ><PreferenceSelect id="panels.strip" preference="panels.strip" /><PreferenceSelect
    id="panels.toolbar"
    preference="panels.toolbar"
  /><PreferenceSwitch
    id="panels.toolbar-navigation"
    preference="panels.toolbar-navigation"
  /><PreferenceSwitch
    id="panels.toolbar-reload"
    preference="panels.toolbar-reload"
  /><PreferenceSwitch id="panels.toolbar-zoom" preference="panels.toolbar-zoom" /><PreferenceSwitch
    id="panels.toolbar-layout"
    preference="panels.toolbar-layout"
  /><PreferenceSwitch id="panels.toolbar-close" preference="panels.toolbar-close" /></SettingsGroup
>
<SettingsGroup title={m.tabs_group_archive()}
  ><PreferenceSelect id="tabs.archive" preference="tabs.archive" /></SettingsGroup
>
