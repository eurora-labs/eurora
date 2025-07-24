# --- ConPort Memory Strategy ---

conport_memory_strategy:

# CRITICAL: At the beginning of every session, the agent MUST execute the 'initialization' sequence

# to determine the ConPort status and load relevant context.

workspace_id_source: "The agent must obtain the absolute path to the current workspace to use as `workspace_id` for all ConPort tool calls. This might be available as `${workspaceFolder}` or require asking the user."

initialization:
thinking_preamble: |

    agent_action_plan:
      - step: 1
        action: "Determine `ACTUAL_WORKSPACE_ID`."
      - step: 2
        action: "Invoke `list_files` for `ACTUAL_WORKSPACE_ID + \"/context_portal/\"`."
        tool_to_use: "list_files"
        parameters: "path: ACTUAL_WORKSPACE_ID + \"/context_portal/\""
      - step: 3
        action: "Analyze result and branch based on 'context.db' existence."
        conditions:
          - if: "'context.db' is found"
            then_sequence: "load_existing_conport_context"
          - else: "'context.db' NOT found"
            then_sequence: "handle_new_conport_setup"

load_existing_conport_context:
thinking_preamble: |

    agent_action_plan:
      - step: 1
        description: "Attempt to load initial contexts from ConPort."
        actions:
          - "Invoke `get_product_context`... Store result."
          - "Invoke `get_active_context`... Store result."
          - "Invoke `get_decisions` (limit 5 for a better overview)... Store result."
          - "Invoke `get_progress` (limit 5)... Store result."
          - "Invoke `get_system_patterns` (limit 5)... Store result."
          - "Invoke `get_custom_data` (category: \"critical_settings\")... Store result."
          - "Invoke `get_custom_data` (category: \"ProjectGlossary\")... Store result."
          - "Invoke `get_recent_activity_summary` (default params, e.g., last 24h, limit 3 per type) for a quick catch-up. Store result."
      - step: 2
        description: "Analyze loaded context."
        conditions:
          - if: "results from step 1 are NOT empty/minimal"
            actions:
              - "Set internal status to [CONPORT_ACTIVE]."
              - "Inform user: \"ConPort memory initialized. Existing contexts and recent activity loaded.\""
              - "Use `ask_followup_question` with suggestions like \"Review recent activity?\", \"Continue previous task?\", \"What would you like to work on?\"."
          - else: "loaded context is empty/minimal despite DB file existing"
            actions:
              - "Set internal status to [CONPORT_ACTIVE]."
              - "Inform user: \"ConPort database file found, but it appears to be empty or minimally initialized. You can start by defining Product/Active Context or logging project information.\""
              - "Use `ask_followup_question` with suggestions like \"Define Product Context?\", \"Log a new decision?\"."
      - step: 3
        description: "Handle Load Failure (if step 1's `get_*` calls failed)."
        condition: "If any `get_*` calls in step 1 failed unexpectedly"
        action: "Fall back to `if_conport_unavailable_or_init_failed`."

handle_new_conport_setup:
thinking_preamble: |

    agent_action_plan:
      - step: 1
        action: "Inform user: \"No existing ConPort database found at `ACTUAL_WORKSPACE_ID + \"/context_portal/context.db\"`.\""
      - step: 2
        action: "Use `ask_followup_question`."
        tool_to_use: "ask_followup_question"
        parameters:
          question: "Would you like to initialize a new ConPort database for this workspace? The database will be created automatically when ConPort tools are first used."
          suggestions:
            - "Yes, initialize a new ConPort database."
            - "No, do not use ConPort for this session."
      - step: 3
        description: "Process user response."
        conditions:
          - if_user_response_is: "Yes, initialize a new ConPort database."
            actions:
              - "Inform user: \"Okay, a new ConPort database will be created.\""
              - description: "Attempt to bootstrap Product Context from projectBrief.md (this happens only on new setup)."
                thinking_preamble: |

                sub_steps:
                  - "Invoke `list_files` with `path: ACTUAL_WORKSPACE_ID` (non-recursive, just to check root)."
                  - description: "Analyze `list_files` result for 'projectBrief.md'."
                    conditions:
                      - if: "'projectBrief.md' is found in the listing"
                        actions:
                          - "Invoke `read_file` for `ACTUAL_WORKSPACE_ID + \"/projectBrief.md\"`."
                          - action: "Use `ask_followup_question`."
                            tool_to_use: "ask_followup_question"
                            parameters:
                              question: "Found projectBrief.md in your workspace. As we're setting up ConPort for the first time, would you like to import its content into the Product Context?"
                              suggestions:
                                - "Yes, import its content now."
                                - "No, skip importing it for now."
                          - description: "Process user response to import projectBrief.md."
                            conditions:
                              - if_user_response_is: "Yes, import its content now."
                                actions:
                                  - "(No need to `get_product_context` as DB is new and empty)"
                                  - "Prepare `content` for `update_product_context`. For example: `{\"initial_product_brief\": \"[content from projectBrief.md]\"}`."
                                  - "Invoke `update_product_context` with the prepared content."
                                  - "Inform user of the import result (success or failure)."
                      - else: "'projectBrief.md' NOT found"
                        actions:
                          - action: "Use `ask_followup_question`."
                            tool_to_use: "ask_followup_question"
                            parameters:
                              question: "`projectBrief.md` was not found in the workspace root. Would you like to define the initial Product Context manually now?"
                              suggestions:
                                - "Define Product Context manually."
                                - "Skip for now."
                          - "(If \"Define manually\", guide user through `update_product_context`)."
              - "Proceed to 'load_existing_conport_context' sequence (which will now load the potentially bootstrapped product context and other empty contexts)."
          - if_user_response_is: "No, do not use ConPort for this session."
            action: "Proceed to `if_conport_unavailable_or_init_failed` (with a message indicating user chose not to initialize)."

if_conport_unavailable_or_init_failed:
thinking_preamble: |

    agent_action: "Inform user: \"ConPort memory will not be used for this session. Status: [CONPORT_INACTIVE].\""

general:
status_prefix: "Begin EVERY response with either '[CONPORT_ACTIVE]' or '[CONPORT_INACTIVE]'."
proactive_logging_cue: "Remember to proactively identify opportunities to log or update ConPort based on the conversation (e.g., if user outlines a new plan, consider logging decisions or progress). Confirm with the user before logging."
proactive_error_handling: "When encountering errors (e.g., tool failures, unexpected output), proactively log the error details using `log_custom_data` (category: 'ErrorLogs', key: 'timestamp_error_summary') and consider updating `active_context` with `open_issues` if it's a persistent problem. Prioritize using ConPort's `get_item_history` or `get_recent_activity_summary` to diagnose issues if they relate to past context changes."
semantic_search_emphasis: "For complex or nuanced queries, especially when direct keyword search (`search_decisions_fts`, `search_custom_data_value_fts`) might be insufficient, prioritize using `semantic_search_conport` to leverage conceptual understanding and retrieve more relevant context. Explain to the user why semantic search is being used."

conport_updates:
frequency: "UPDATE CONPORT THROUGHOUT THE CHAT SESSION, WHEN SIGNIFICANT CHANGES OCCUR, OR WHEN EXPLICITLY REQUESTED."
workspace_id_note: "All ConPort tool calls require the `workspace_id`."
tools: - name: get_product_context
trigger: "To understand the overall project goals, features, or architecture at any time."
action_description: | # Agent Action: Invoke `get_product_context` (`{"workspace_id": "..."}`). Result is a direct dictionary. - name: update_product_context
trigger: "When the high-level project description, goals, features, or overall architecture changes significantly, as confirmed by the user."
action_description: |
<thinking> - Product context needs updating. - Step 1: (Optional but recommended if unsure of current state) Invoke `get_product_context`. - Step 2: Prepare the `content` (for full overwrite) or `patch_content` (partial update) dictionary. - To remove a key using `patch_content`, set its value to the special string sentinel `\"__DELETE__\"`. - Confirm changes with the user.
</thinking> # Agent Action: Invoke `update_product_context` (`{"workspace_id": "...", "content": {...}}` or `{"workspace_id": "...", "patch_content": {"key_to_update": "new_value", "key_to_delete": "__DELETE__"}}`). - name: get_active_context
trigger: "To understand the current task focus, immediate goals, or session-specific context."
action_description: | # Agent Action: Invoke `get_active_context` (`{"workspace_id": "..."}`). Result is a direct dictionary. - name: update_active_context
trigger: "When the current focus of work changes, new questions arise, or session-specific context needs updating (e.g., `current_focus`, `open_issues`), as confirmed by the user."
action_description: |
<thinking> - Active context needs updating. - Step 1: (Optional) Invoke `get_active_context` to retrieve the current state. - Step 2: Prepare `content` (for full overwrite) or `patch_content` (for partial update). - Common fields to update include `current_focus`, `open_issues`, and other session-specific data. - To remove a key using `patch_content`, set its value to the special string sentinel `\"__DELETE__\"`. - Confirm changes with the user.
</thinking> # Agent Action: Invoke `update_active_context` (`{"workspace_id": "...", "content": {...}}` or `{"workspace_id": "...", "patch_content": {"current_focus": "new_focus", "open_issues": ["issue1", "issue2"], "key_to_delete": "__DELETE__"}}`). - name: log_decision
trigger: "When a significant architectural or implementation decision is made and confirmed by the user."
action_description: | # Agent Action: Invoke `log_decision` (`{"workspace_id": "...", "summary": "...", "rationale": "...", "tags": ["optional_tag"]}}`). - name: get_decisions
trigger: "To retrieve a list of past decisions, e.g., to review history or find a specific decision."
action_description: | # Agent Action: Invoke `get_decisions` (`{"workspace_id": "...", "limit": N, "tags_filter_include_all": ["tag1"], "tags_filter_include_any": ["tag2"]}}`). Explain optional filters. - name: search_decisions_fts
trigger: "When searching for decisions by keywords in summary, rationale, details, or tags, and basic `get_decisions` is insufficient."
action_description: | # Agent Action: Invoke `search_decisions_fts` (`{"workspace_id": "...", "query_term": "search keywords", "limit": N}}`). - name: delete_decision_by_id
trigger: "When user explicitly confirms deletion of a specific decision by its ID."
action_description: | # Agent Action: Invoke `delete_decision_by_id` (`{"workspace_id": "...", "decision_id": ID}}`). Emphasize prior confirmation. - name: log_progress
trigger: "When a task begins, its status changes (e.g., TODO, IN_PROGRESS, DONE), or it's completed. Also when a new sub-task is defined."
action_description: | # Agent Action: Invoke `log_progress` (`{"workspace_id": "...", "description": "...", "status": "...", "linked_item_type": "...", "linked_item_id": "..."}}`). Note: 'summary' was changed to 'description' for log_progress. - name: get_progress
trigger: "To review current task statuses, find pending tasks, or check history of progress."
action_description: | # Agent Action: Invoke `get_progress` (`{"workspace_id": "...", "status_filter": "...", "parent_id_filter": ID, "limit": N}}`). - name: update_progress
trigger: "Updates an existing progress entry."
action_description: | # Agent Action: Invoke `update_progress` (`{"workspace_id": "...", "progress_id": ID, "status": "...", "description": "...", "parent_id": ID}}`). - name: delete_progress_by_id
trigger: "Deletes a progress entry by its ID."
action_description: | # Agent Action: Invoke `delete_progress_by_id` (`{"workspace_id": "...", "progress_id": ID}}`). - name: log_system_pattern
trigger: "When new architectural patterns are introduced, or existing ones are modified, as confirmed by the user."
action_description: | # Agent Action: Invoke `log_system_pattern` (`{"workspace_id": "...", "name": "...", "description": "...", "tags": ["optional_tag"]}}`). - name: get_system_patterns
trigger: "To retrieve a list of defined system patterns."
action_description: | # Agent Action: Invoke `get_system_patterns` (`{"workspace_id": "...", "tags_filter_include_all": ["tag1"], "limit": N}}`). Note: limit was not in original example, added for consistency. - name: delete_system_pattern_by_id
trigger: "When user explicitly confirms deletion of a specific system pattern by its ID."
action_description: | # Agent Action: Invoke `delete_system_pattern_by_id` (`{"workspace_id": "...", "pattern_id": ID}}`). Emphasize prior confirmation. - name: log_custom_data
trigger: "To store any other type of structured or unstructured project-related information not covered by other tools (e.g., glossary terms, technical specs, meeting notes), as confirmed by the user."
action_description: | # Agent Action: Invoke `log_custom_data` (`{"workspace_id": "...", "category": "...", "key": "...", "value": {... or "string"}}`). Note: 'metadata' field is not part of log_custom_data args. - name: get_custom_data
trigger: "To retrieve specific custom data by category and key."
action_description: | # Agent Action: Invoke `get_custom_data` (`{"workspace_id": "...", "category": "...", "key": "..."}}`). - name: delete_custom_data
trigger: "When user explicitly confirms deletion of specific custom data by category and key."
action_description: | # Agent Action: Invoke `delete_custom_data` (`{"workspace_id": "...", "category": "...", "key": "..."}}`). Emphasize prior confirmation. - name: search_custom_data_value_fts
trigger: "When searching for specific terms within any custom data values, categories, or keys."
action_description: | # Agent Action: Invoke `search_custom_data_value_fts` (`{"workspace_id": "...", "query_term": "...", "category_filter": "...", "limit": N}}`). - name: search_project_glossary_fts
trigger: "When specifically searching for terms within the 'ProjectGlossary' custom data category."
action_description: | # Agent Action: Invoke `search_project_glossary_fts` (`{"workspace_id": "...", "query_term": "...", "limit": N}}`). - name: semantic_search_conport
trigger: "When a natural language query requires conceptual understanding beyond keyword matching, or when direct keyword searches are insufficient."
action_description: | # Agent Action: Invoke `semantic_search_conport` (`{"workspace_id": "...", "query_text": "...", "top_k": N, "filter_item_types": ["decision", "custom_data"]}}`). Explain filters. - name: link_conport_items
trigger: "When a meaningful relationship is identified and confirmed between two existing ConPort items (e.g., a decision is implemented by a system pattern, a progress item tracks a decision)."
action_description: |
<thinking> - Need to link two items. Identify source type/ID, target type/ID, and relationship. - Common relationship_types: 'implements', 'related_to', 'tracks', 'blocks', 'clarifies', 'depends_on'. Propose a suitable one or ask user.
</thinking> # Agent Action: Invoke `link_conport_items` (`{"workspace_id":"...", "source_item_type":"...", "source_item_id":"...", "target_item_type":"...", "target_item_id":"...", "relationship_type":"...", "description":"Optional notes"}`). - name: get_linked_items
trigger: "To understand the relationships of a specific ConPort item, or to explore the knowledge graph around an item."
action_description: | # Agent Action: Invoke `get_linked_items` (`{"workspace_id":"...", "item_type":"...", "item_id":"...", "relationship_type_filter":"...", "linked_item_type_filter":"...", "limit":N}`). - name: get_item_history
trigger: "When needing to review past versions of Product Context or Active Context, or to see when specific changes were made."
action_description: | # Agent Action: Invoke `get_item_history` (`{"workspace_id":"...", "item_type":"product_context" or "active_context", "limit":N, "version":V, "before_timestamp":"ISO_DATETIME", "after_timestamp":"ISO_DATETIME"}`). - name: batch_log_items
trigger: "When the user provides a list of multiple items of the SAME type (e.g., several decisions, multiple new glossary terms) to be logged at once."
action_description: |
<thinking> - User provided multiple items. Verify they are of the same loggable type. - Construct the `items` list, where each element is a dictionary of arguments for the single-item log tool (e.g., for `log_decision`).
</thinking> # Agent Action: Invoke `batch_log_items` (`{"workspace_id":"...", "item_type":"decision", "items": [{"summary":"...", "rationale":"..."}, {"summary":"..."}] }`). - name: get_recent_activity_summary
trigger: "At the start of a new session to catch up, or when the user asks for a summary of recent project activities."
action_description: | # Agent Action: Invoke `get_recent_activity_summary` (`{"workspace_id":"...", "hours_ago":H, "since_timestamp":"ISO_DATETIME", "limit_per_type":N}`). Explain default if no time args. - name: get_conport_schema
trigger: "If there's uncertainty about available ConPort tools or their arguments during a session (internal LLM check), or if an advanced user specifically asks for the server's tool schema."
action_description: | # Agent Action: Invoke `get_conport_schema` (`{"workspace_id":"..."}`). Primarily for internal LLM reference or direct user request. - name: export_conport_to_markdown
trigger: "When the user requests to export the current ConPort data to markdown files (e.g., for backup, sharing, or version control)."
action_description: | # Agent Action: Invoke `export_conport_to_markdown` (`{"workspace_id":"...", "output_path":"optional/relative/path"}`). Explain default output path if not provided. - name: import_markdown_to_conport
trigger: "When the user requests to import ConPort data from a directory of markdown files previously exported by this system."
action_description: | # Agent Action: Invoke `import_markdown_to_conport` (`{"workspace_id":"...", "input_path":"optional/relative/path"}`). Explain default input path. Warn about potential overwrites or merges if data already exists. - name: reconfigure_core_guidance
type: guidance
product_active_context: "The internal JSON structure of 'Product Context' and 'Active Context' (the `content` field) is flexible. Work with the user to define and evolve this structure via `update_product_context` and `update_active_context`. The server stores this `content` as a JSON blob."
decisions_progress_patterns: "The fundamental fields for Decisions, Progress, and System Patterns are fixed by ConPort's tools. For significantly different structures or additional fields, guide the user to create a new custom context category using `log_custom_data` (e.g., category: 'project_milestones_detailed')."

conport*sync_routine:
trigger: "^(Sync ConPort|ConPort Sync)$"
user_acknowledgement_text: "[CONPORT_SYNCING]"
instructions: - "Halt Current Task: Stop current activity." - "Acknowledge Command: Send `[CONPORT_SYNCING]` to the user." - "Review Chat History: Analyze the complete current chat session for new information, decisions, progress, context changes, clarifications, and potential new relationships between items."
core_update_process:
thinking_preamble: | - Synchronize ConPort with information from the current chat session. - Use appropriate ConPort tools based on identified changes. - For `update_product_context` and `update_active_context`, first fetch current content, then merge/update (potentially using `patch_content`), then call the update tool with the \_complete new content object* or the patch. - All tool calls require the `workspace_id`.
agent_action_plan_illustrative: - "Log new decisions (use `log_decision`)." - "Log task progress/status changes (use `log_progress`)." - "Update existing progress entries (use `update_progress`)." - "Delete progress entries (use `delete_progress_by_id`)." - "Log new system patterns (use `log_system_pattern`)." - "Update Active Context (use `get_active_context` then `update_active_context` with full or patch)." - "Update Product Context if significant changes (use `get_product_context` then `update_product_context` with full or patch)." - "Log new custom context, including ProjectGlossary terms (use `log_custom_data`)." - "Identify and log new relationships between items (use `link_conport_items`)." - "If many items of the same type were discussed, consider `batch_log_items`." - "After updates, consider a brief `get_recent_activity_summary` to confirm and refresh understanding."
post_sync_actions: - "Inform user: ConPort synchronized with session info." - "Resume previous task or await new instructions."

dynamic_context_retrieval_for_rag:
description: |
Guidance for dynamically retrieving and assembling context from ConPort to answer user queries or perform tasks,
enhancing Retrieval Augmented Generation (RAG) capabilities.
trigger: "When the AI needs to answer a specific question, perform a task requiring detailed project knowledge, or generate content based on ConPort data."
goal: "To construct a concise, highly relevant context set for the LLM, improving the accuracy and relevance of its responses."
steps: - step: 1
action: "Analyze User Query/Task"
details: "Deconstruct the user's request to identify key entities, concepts, keywords, and the specific type of information needed from ConPort." - step: 2
action: "Prioritized Retrieval Strategy"
details: |
Based on the analysis, select the most appropriate ConPort tools: - **Targeted FTS:** Use `search_decisions_fts`, `search_custom_data_value_fts`, `search_project_glossary_fts` for keyword-based searches if specific terms are evident. - **Specific Item Retrieval:** Use `get_custom_data` (if category/key known), `get_decisions` (by ID or for recent items), `get_system_patterns`, `get_progress` if the query points to specific item types or IDs. - **(Future):** Prioritize semantic search tools once available for conceptual queries. - **Broad Context (Fallback):** Use `get_product_context` or `get_active_context` as a fallback if targeted retrieval yields little, but be mindful of their size. - step: 3
action: "Retrieve Initial Set"
details: "Execute the chosen tool(s) to retrieve an initial, small set (e.g., top 3-5) of the most relevant items or data snippets." - step: 4
action: "Contextual Expansion (Optional)"
details: "For the most promising items from Step 3, consider using `get_linked_items` to fetch directly related items (1-hop). This can provide crucial context or disambiguation. Use judiciously to avoid excessive data." - step: 5
action: "Synthesize and Filter"
details: |
Review the retrieved information (initial set + expanded context). - **Filter:** Discard irrelevant items or parts of items. - **Synthesize/Summarize:** If multiple relevant pieces of information are found, synthesize them into a concise summary that directly addresses the query/task. Extract only the most pertinent sentences or facts. - step: 6
action: "Assemble Prompt Context"
details: |
Construct the context portion of the LLM prompt using the filtered and synthesized information. - **Clarity:** Clearly delineate this retrieved context from the user's query or other parts of the prompt. - **Attribution (Optional but Recommended):** If possible, briefly note the source of the information (e.g., "From Decision D-42:", "According to System Pattern SP-5:"). - **Brevity:** Strive for relevance and conciseness. Avoid including large, unprocessed chunks of data unless absolutely necessary and directly requested.
general_principles: - "Prefer targeted retrieval over broad context dumps." - "Iterate if initial retrieval is insufficient: try different keywords or tools." - "Balance context richness with prompt token limits."

proactive_knowledge_graph_linking:
description: |
Guidance for the AI to proactively identify and suggest the creation of links between ConPort items,
enriching the project's knowledge graph based on conversational context.
trigger: "During ongoing conversation, when the AI observes potential relationships (e.g., causal, implementational, clarifying) between two or more discussed ConPort items or concepts that are likely represented as ConPort items."
goal: "To actively build and maintain a rich, interconnected knowledge graph within ConPort by capturing relationships that might otherwise be missed."
steps: - step: 1
action: "Monitor Conversational Context"
details: "Continuously analyze the user's statements and the flow of discussion for mentions of ConPort items (explicitly by ID, or implicitly by well-known names/summaries) and the relationships being described or implied between them." - step: 2
action: "Identify Potential Links"
details: |
Look for patterns such as: - User states "Decision X led to us doing Y (which is Progress item P-3)." - User discusses how System Pattern SP-2 helps address a concern noted in Decision D-5. - User outlines a task (Progress P-10) that implements a specific feature detailed in a `custom_data` spec (CD-Spec-FeatureX). - step: 3
action: "Formulate and Propose Link Suggestion"
details: |
If a potential link is identified: - Clearly state the items involved (e.g., "Decision D-5", "System Pattern SP-2"). - Describe the perceived relationship (e.g., "It seems SP-2 addresses a concern in D-5."). - Propose creating a link using `ask_followup_question`. - Example Question: "I noticed we're discussing Decision D-5 and System Pattern SP-2. It sounds like SP-2 might 'address_concern_in' D-5. Would you like me to create this link in ConPort? You can also suggest a different relationship type." - Suggested Answers: - "Yes, link them with 'addresses_concern_in'." - "Yes, but use relationship type: [user types here]." - "No, don't link them now." - Offer common relationship types as examples if needed: 'implements', 'clarifies', 'related_to', 'depends_on', 'blocks', 'resolves', 'derived_from'. - step: 4
action: "Gather Details and Execute Linking"
details: |
If the user confirms: - Ensure you have the correct source item type, source item ID, target item type, target item ID, and the agreed-upon relationship type. - Ask for an optional brief description for the link if the relationship isn't obvious. - Invoke the `link_conport_items` tool. - step: 5
action: "Confirm Outcome"
details: "Inform the user of the success or failure of the `link_conport_items` tool call."
general_principles: - "Be helpful, not intrusive. If the user declines a suggestion, accept and move on." - "Prioritize clear, strong relationships over tenuous ones." - "This strategy complements the general `proactive_logging_cue` by providing specific guidance for link creation."
