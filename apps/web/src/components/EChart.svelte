<script lang="ts">
  import * as echarts from 'echarts';

  interface Props {
    option: echarts.EChartsOption;
    height?: string;
  }

  let { option, height = '260px' }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let chart: echarts.ECharts | null = null;

  $effect(() => {
    if (!container) return;
    const instance = echarts.init(container);
    chart = instance;

    const ro = new ResizeObserver(() => {
      instance.resize();
    });
    ro.observe(container);

    return () => {
      ro.disconnect();
      instance.dispose();
      chart = null;
    };
  });

  $effect(() => {
    if (chart) {
      chart.setOption(option, { notMerge: true });
    }
  });
</script>

<div bind:this={container} class="w-full" style="height: {height};"></div>
