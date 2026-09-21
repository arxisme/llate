<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3Force from 'd3-force';
  import * as d3Selection from 'd3-selection';
  import * as d3Drag from 'd3-drag';

  let { data, onNodeClick, onClose } = $props<{
    data: { nodes: string[], links: [string, string][] },
    onNodeClick: (node: string) => void,
    onClose: () => void
  }>();
  
  let svgRef: SVGSVGElement;

  onMount(() => {
    const width = 800;
    const height = 600;

    const nodes = data.nodes.map(id => ({ id, x: 0, y: 0, vx: 0, vy: 0 }));
    const links = data.links.map(([source, target]) => ({ source, target }));

    const simulation = d3Force.forceSimulation(nodes as any)
      .force("link", d3Force.forceLink(links).id((d: any) => d.id).distance(150))
      .force("charge", d3Force.forceManyBody().strength(-400))
      .force("center", d3Force.forceCenter(width / 2, height / 2));

    const svg = d3Selection.select(svgRef)
      .attr("viewBox", [0, 0, width, height]);

    const link = svg.append("g")
      .attr("stroke", "var(--outline-variant)")
      .attr("stroke-opacity", 1)
      .selectAll("line")
      .data(links)
      .join("line")
      .attr("stroke-width", 1.5);

    const node = svg.append("g")
      .attr("stroke", "var(--primary-container)")
      .attr("stroke-width", 2)
      .selectAll("circle")
      .data(nodes)
      .join("circle")
      .attr("r", 6)
      .attr("fill", "var(--primary)")
      .style("cursor", "pointer")
      .style("filter", "drop-shadow(0 0 8px var(--primary-container))")
      .on("click", (event: any, d: any) => {
        onNodeClick(d.id);
      })
      .call(drag(simulation) as any);

    const text = svg.append("g")
      .selectAll("text")
      .data(nodes)
      .join("text")
      .text((d: any) => d.id)
      .attr("font-size", 11)
      .attr("font-family", "var(--font-ui)")
      .attr("fill", "var(--on-surface)")
      .attr("dx", 12)
      .attr("dy", 4)
      .style("pointer-events", "none");

    simulation.on("tick", () => {
      link
        .attr("x1", (d: any) => d.source.x)
        .attr("y1", (d: any) => d.source.y)
        .attr("x2", (d: any) => d.target.x)
        .attr("y2", (d: any) => d.target.y);

      node
        .attr("cx", (d: any) => d.x)
        .attr("cy", (d: any) => d.y);

      text
        .attr("x", (d: any) => d.x)
        .attr("y", (d: any) => d.y);
    });

    function drag(simulation: any) {
      function dragstarted(event: any) {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        event.subject.fx = event.subject.x;
        event.subject.fy = event.subject.y;
      }
      
      function dragged(event: any) {
        event.subject.fx = event.x;
        event.subject.fy = event.y;
      }
      
      function dragended(event: any) {
        if (!event.active) simulation.alphaTarget(0);
        event.subject.fx = null;
        event.subject.fy = null;
      }
      
      return d3Drag.drag()
        .on("start", dragstarted)
        .on("drag", dragged)
        .on("end", dragended);
    }
  });
</script>

<div class="graph-fullscreen-overlay">
  <div class="graph-header">
    <div class="graph-title">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="graph-icon"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg>
      <h2>Knowledge Graph</h2>
    </div>
    <button class="close-btn" onclick={onClose}>
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </button>
  </div>
  
  <div class="graph-body">
    <svg bind:this={svgRef} width="100%" height="100%"></svg>
  </div>
</div>

<style>
  .graph-fullscreen-overlay {
    position: fixed;
    top: 36px; /* offset by titlebar */
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(18, 19, 26, 0.8); /* Derived from --background */
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    animation: fadeIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }
  
  @keyframes fadeIn {
    from { opacity: 0; backdrop-filter: blur(0px); }
    to { opacity: 1; backdrop-filter: blur(24px); }
  }

  .graph-header {
    display: flex;
    justify-content: space-between;
    padding: 16px 24px;
    align-items: center;
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10;
    pointer-events: none;
    border-bottom: 1px solid var(--outline-variant);
    background: transparent;
  }

  .graph-title {
    display: flex;
    align-items: center;
    gap: 12px;
    pointer-events: auto;
  }

  .graph-icon {
    color: var(--primary);
    filter: drop-shadow(0 0 8px var(--primary-container));
  }

  .graph-title h2 {
    margin: 0;
    color: var(--on-surface);
    font-size: 1.2rem;
    font-weight: 500;
    letter-spacing: -0.01em;
    font-family: var(--font-ui);
  }

  .close-btn {
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid transparent;
    padding: 10px;
    border-radius: var(--radius-md);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    pointer-events: auto;
  }

  .close-btn:hover {
    background: var(--surface-container-high);
    color: var(--on-surface);
  }

  .graph-body {
    flex: 1;
    width: 100%;
    height: 100%;
    position: relative;
    cursor: grab;
  }

  .graph-body:active {
    cursor: grabbing;
  }
</style>
