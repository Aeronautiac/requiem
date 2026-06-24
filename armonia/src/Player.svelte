<script lang="ts">
  import Button from "$lib/components/ui/button/button.svelte";

  interface Props {
    id: number;
    display_name: string;
  }
  let { id, display_name }: Props = $props();
</script>

<div>
  <Button>display_name</Button>
</div>
