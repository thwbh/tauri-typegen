<script>
  import Greet from './lib/Greet.svelte'
  // Using generated TypeScript bindings - type-safe with validation!
  import { ping, analyzeCommands, generateModels, greet } from './generated/commands'
  import type { AnalyzeCommandsParams, GenerateModelsParams } from './generated/types'

	let response = ''
	let analyzing = false
	let generating = false

	function updateResponse(returnValue) {
		response += `[${new Date().toLocaleTimeString()}] ` + (typeof returnValue === 'string' ? returnValue : JSON.stringify(returnValue, null, 2)) + '<br>'
	}

	function _ping() {
		// Type-safe call with automatic validation
		ping({ value: "Hello from generated bindings!" })
			.then(updateResponse)
			.catch(updateResponse)
	}

	function _greet() {
		// Demonstrating the generated greet command
		greet({ name: "TypeScript" })
			.then(updateResponse)
			.catch(updateResponse)
	}

	async function _analyzeCommands() {
		analyzing = true
		try {
			// Type-safe parameters with IntelliSense support
			const params: AnalyzeCommandsParams = {
				projectPath: "../librefit/src-tauri"
			}
			
			const result = await analyzeCommands(params)
			// result is properly typed as AnalyzeCommandsResponse
			updateResponse(`Found ${result.commands.length} commands: ${result.commands.map(c => c.name).join(', ')}`)
		} catch (error) {
			updateResponse(`Analysis failed: ${error}`)
		} finally {
			analyzing = false
		}
	}

	async function _generateModels() {
		generating = true
		try {
			// Type-safe parameters with full IntelliSense
			const params: GenerateModelsParams = {
				projectPath: "../librefit/src-tauri",
				outputPath: "../librefit/generated-models",
				validationLibrary: "zod"
			}
			
			const result = await generateModels(params)
			// result is properly typed as GenerateModelsResponse
			updateResponse(`Generated ${result.generatedFiles.length} files for ${result.commandsFound} commands`)
		} catch (error) {
			updateResponse(`Generation failed: ${error}`)
		} finally {
			generating = false
		}
	}
</script>

<main class="container">
  <h1>Welcome to Tauri!</h1>

  <div class="row">
    <a href="https://vitejs.dev" target="_blank">
      <img src="/vite.svg" class="logo vite" alt="Vite Logo" />
    </a>
    <a href="https://tauri.app" target="_blank">
      <img src="/tauri.svg" class="logo tauri" alt="Tauri Logo" />
    </a>
    <a href="https://svelte.dev" target="_blank">
      <img src="/svelte.svg" class="logo svelte" alt="Svelte Logo" />
    </a>
  </div>

  <p>
    Click on the Tauri, Vite, and Svelte logos to learn more.
  </p>

  <div class="row">
    <Greet />
  </div>

  <div>
    <h2>Tauri Plugin TypeGen Demo</h2>
    <p><strong>✨ All functions below use generated TypeScript bindings with type safety and validation!</strong></p>
    
    <div class="button-group">
      <button on:click="{_greet}">🎉 Greet (Generated)</button>
      <button on:click="{_ping}">📡 Test Ping</button>
    </div>
    
    <div class="button-group">
      <button on:click="{_analyzeCommands}" disabled={analyzing}>
        {analyzing ? 'Analyzing...' : '🔍 Analyze LibreFit Commands'}
      </button>
      <button on:click="{_generateModels}" disabled={generating}>
        {generating ? 'Generating...' : '⚡ Generate TypeScript Models'}
      </button>
    </div>

    <div class="info-box">
      <h3>💡 How this works:</h3>
      <ol>
        <li>Run: <code>cargo tauri-typegen generate</code></li>
        <li>TypeScript files are generated in <code>./src/generated/</code></li>
        <li>Import type-safe functions: <code>import &#123; greet &#125; from './generated'</code></li>
        <li>Enjoy IntelliSense and runtime validation! 🎯</li>
      </ol>
    </div>

    <div class="response-log">{@html response}</div>
  </div>

</main>

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }

  .button-group {
    display: flex;
    gap: 10px;
    margin: 20px 0;
  }

  .button-group button {
    padding: 8px 16px;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f5f5f5;
    cursor: pointer;
  }

  .button-group button:disabled {
    background: #e0e0e0;
    cursor: not-allowed;
  }

  .button-group button:hover:not(:disabled) {
    background: #e0e0e0;
  }

  .info-box {
    background: #e8f5e8;
    border: 1px solid #4caf50;
    border-radius: 6px;
    padding: 15px;
    margin: 20px 0;
  }
  
  .info-box h3 {
    margin: 0 0 10px 0;
    color: #2e7d2e;
  }
  
  .info-box ol {
    margin: 0;
    padding-left: 20px;
  }
  
  .info-box li {
    margin: 5px 0;
  }
  
  .info-box code {
    background: #fff;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'Courier New', monospace;
    font-size: 13px;
  }

  .response-log {
    background: #f5f5f5;
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 10px;
    margin: 20px 0;
    max-height: 300px;
    overflow-y: auto;
    font-family: monospace;
    font-size: 12px;
  }
</style>
