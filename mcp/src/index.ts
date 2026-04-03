/**
 * Knight Agent MCP Adapter
 *
 * Design Reference: docs/03-module-design/infrastructure/mcp-adapter.md
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";

/**
 * MCP Server implementation for Knight Agent
 */
export class KnightAgentMCPServer {
  private server: Server;

  constructor() {
    this.server = new Server(
      {
        name: "knight-agent",
        version: "0.1.0",
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.setupHandlers();
  }

  private setupHandlers(): void {
    // Handle tool listing
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: this.getTools(),
      };
    });

    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          case "create_session":
            return { content: [{ type: "text", text: JSON.stringify({ session_id: "mock-session-id" }) }] };
          case "execute_agent":
            return { content: [{ type: "text", text: JSON.stringify({ result: "mock-result" }) }] };
          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error) {
        return {
          content: [{ type: "text", text: `Error: ${error}` }],
          isError: true,
        };
      }
    });
  }

  private getTools(): Tool[] {
    return [
      {
        name: "create_session",
        description: "Create a new Knight Agent session",
        inputSchema: {
          type: "object",
          properties: {
            config: {
              type: "object",
              description: "Session configuration",
            },
          },
        },
      },
      {
        name: "execute_agent",
        description: "Execute an agent task",
        inputSchema: {
          type: "object",
          properties: {
            task: {
              type: "string",
              description: "Task description",
            },
          },
        },
      },
    ];
  }

  async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
  }
}

// Main entry point
if (require.main === module) {
  const server = new KnightAgentMCPServer();
  server.start().catch(console.error);
}

export default KnightAgentMCPServer;
