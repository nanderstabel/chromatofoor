use anyhow::Result;
use esdm::EsdmDoc;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct GraphDomain {
    name: String,
    description: Option<String>,
}

#[derive(Serialize)]
struct GraphNode {
    id: String,
    name: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounded_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
struct GraphLink {
    source: String,
    target: String,
    #[serde(rename = "type")]
    link_type: String,
}

#[derive(Serialize)]
struct DomainGraph {
    domain: GraphDomain,
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
}

pub fn emit_graph_file(
    docs: &[EsdmDoc],
    event_to_owner: &HashMap<String, (String, String)>,
    output_path: &std::path::Path,
) -> Result<()> {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    let domain_doc = docs.iter().find(|d| d.kind == "domain");
    let domain_name = domain_doc.map_or_else(|| "global".to_string(), |d| d.name.clone());
    let domain_desc = domain_doc.and_then(|d| d.description.clone());

    // Add Domain Node
    nodes.push(GraphNode {
        id: format!("domain:{domain_name}"),
        name: domain_name.clone(),
        kind: "domain".to_string(),
        bounded_context: None,
        description: domain_desc.clone(),
    });

    let get_bc = |doc: &EsdmDoc| -> String {
        doc.scope
            .as_ref()
            .and_then(|s| s.bounded_context.clone())
            .unwrap_or_else(|| "global".to_string())
    };

    let get_actor_id = |bc: &str, actor_name: &str, all_docs: &[EsdmDoc]| -> String {
        let exists_in_bc = all_docs
            .iter()
            .any(|d| d.kind == "actor" && d.name == actor_name && get_bc(d) == bc);
        if exists_in_bc {
            format!("actor:{bc}:{actor_name}")
        } else if let Some(act_doc) = all_docs
            .iter()
            .find(|d| d.kind == "actor" && d.name == actor_name)
        {
            format!("actor:{}:{}", get_bc(act_doc), actor_name)
        } else {
            format!("actor:global:{actor_name}")
        }
    };

    let resolve_event_bc = |cmd_bc: &str,
                            event_name: &str,
                            all_docs: &[EsdmDoc],
                            ev_to_owner: &HashMap<String, (String, String)>|
     -> String {
        if let Some((mapped_bc, _)) = ev_to_owner.get(event_name) {
            mapped_bc.clone()
        } else if let Some(ev_doc) = all_docs
            .iter()
            .find(|d| d.kind == "event" && d.name == event_name)
        {
            get_bc(ev_doc)
        } else {
            cmd_bc.to_string()
        }
    };

    for doc in docs {
        let doc_bc = get_bc(doc);
        match doc.kind.as_str() {
            "domain" => {}
            "bounded-context" => {
                let bc_id = format!("bounded-context:{}", doc.name);
                nodes.push(GraphNode {
                    id: bc_id.clone(),
                    name: doc.name.clone(),
                    kind: "bounded-context".to_string(),
                    bounded_context: None,
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("domain:{domain_name}"),
                    target: bc_id,
                    link_type: "contains".to_string(),
                });
            }
            "actor" => {
                let actor_id = format!("actor:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: actor_id.clone(),
                    name: doc.name.clone(),
                    kind: "actor".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: actor_id,
                    link_type: "contains".to_string(),
                });
            }
            "aggregate" => {
                let agg_id = format!("aggregate:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: agg_id.clone(),
                    name: doc.name.clone(),
                    kind: "aggregate".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: agg_id,
                    link_type: "contains".to_string(),
                });
            }
            "dynamic-consistency-boundary" => {
                let boundary_id = format!("boundary:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: boundary_id.clone(),
                    name: doc.name.clone(),
                    kind: "dynamic-consistency-boundary".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: boundary_id.clone(),
                    link_type: "contains".to_string(),
                });

                if let Some(consults) = &doc.consults {
                    for consult in consults {
                        let target_bc = consult
                            .bounded_context
                            .clone()
                            .unwrap_or_else(|| doc_bc.clone());
                        if let Some(ref event_name) = consult.event {
                            links.push(GraphLink {
                                source: boundary_id.clone(),
                                target: format!("event:{target_bc}:{event_name}"),
                                link_type: "consults".to_string(),
                            });
                        }
                    }
                }
            }
            "command" => {
                let cmd_id = format!("command:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: cmd_id.clone(),
                    name: doc.name.clone(),
                    kind: "command".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: cmd_id.clone(),
                    link_type: "contains".to_string(),
                });

                if let Some(ref scope) = doc.scope {
                    if let Some(ref agg) = scope.aggregate {
                        links.push(GraphLink {
                            source: format!("aggregate:{doc_bc}:{agg}"),
                            target: cmd_id.clone(),
                            link_type: "handles".to_string(),
                        });
                    } else if let Some(ref boundary) = scope.dynamic_consistency_boundary {
                        links.push(GraphLink {
                            source: format!("boundary:{doc_bc}:{boundary}"),
                            target: cmd_id.clone(),
                            link_type: "handles".to_string(),
                        });
                    }
                }

                if let Some(publishes) = &doc.publishes {
                    for ev in publishes {
                        let ev_bc = resolve_event_bc(&doc_bc, ev, docs, event_to_owner);
                        links.push(GraphLink {
                            source: cmd_id.clone(),
                            target: format!("event:{ev_bc}:{ev}"),
                            link_type: "publishes".to_string(),
                        });
                    }
                }

                if let Some(actors) = &doc.actors {
                    for actor in actors {
                        let act_id = get_actor_id(&doc_bc, actor, docs);
                        links.push(GraphLink {
                            source: act_id,
                            target: cmd_id.clone(),
                            link_type: "invokes".to_string(),
                        });
                    }
                }
            }
            "event" => {
                let ev_id = format!("event:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: ev_id.clone(),
                    name: doc.name.clone(),
                    kind: "event".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: ev_id.clone(),
                    link_type: "contains".to_string(),
                });

                let mut explicit_mapped = false;
                if let Some(ref scope) = doc.scope {
                    if let Some(ref agg) = scope.aggregate {
                        links.push(GraphLink {
                            source: format!("aggregate:{doc_bc}:{agg}"),
                            target: ev_id.clone(),
                            link_type: "emits".to_string(),
                        });
                        explicit_mapped = true;
                    } else if let Some(ref boundary) = scope.dynamic_consistency_boundary {
                        links.push(GraphLink {
                            source: format!("boundary:{doc_bc}:{boundary}"),
                            target: ev_id.clone(),
                            link_type: "emits".to_string(),
                        });
                        explicit_mapped = true;
                    }
                }

                if !explicit_mapped && let Some((_, mapped_agg)) = event_to_owner.get(&doc.name) {
                    let is_boundary = docs.iter().any(|d| {
                        d.kind == "dynamic-consistency-boundary"
                            && &d.name == mapped_agg
                            && get_bc(d) == doc_bc
                    });
                    let source_prefix = if is_boundary { "boundary" } else { "aggregate" };
                    links.push(GraphLink {
                        source: format!("{source_prefix}:{doc_bc}:{mapped_agg}"),
                        target: ev_id,
                        link_type: "emits".to_string(),
                    });
                }
            }
            "read-model" => {
                let rm_id = format!("read-model:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: rm_id.clone(),
                    name: doc.name.clone(),
                    kind: "read-model".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: rm_id.clone(),
                    link_type: "contains".to_string(),
                });

                if let Some(projections) = &doc.projections {
                    for proj in projections {
                        let proj_bc = proj
                            .bounded_context
                            .clone()
                            .unwrap_or_else(|| doc_bc.clone());
                        if let Some(ref event_name) = proj.event {
                            links.push(GraphLink {
                                source: format!("event:{proj_bc}:{event_name}"),
                                target: rm_id.clone(),
                                link_type: "projects".to_string(),
                            });
                        }
                    }
                }
            }
            "query" => {
                let q_id = format!("query:{}:{}", doc_bc, doc.name);
                nodes.push(GraphNode {
                    id: q_id.clone(),
                    name: doc.name.clone(),
                    kind: "query".to_string(),
                    bounded_context: Some(doc_bc.clone()),
                    description: doc.description.clone(),
                });
                links.push(GraphLink {
                    source: format!("bounded-context:{doc_bc}"),
                    target: q_id.clone(),
                    link_type: "contains".to_string(),
                });

                if let Some(ref rm) = doc.read_model {
                    links.push(GraphLink {
                        source: q_id.clone(),
                        target: format!("read-model:{doc_bc}:{rm}"),
                        link_type: "reads".to_string(),
                    });
                }

                if let Some(actors) = &doc.actors {
                    for actor in actors {
                        let act_id = get_actor_id(&doc_bc, actor, docs);
                        links.push(GraphLink {
                            source: act_id,
                            target: q_id.clone(),
                            link_type: "invokes".to_string(),
                        });
                    }
                }
            }
            "context-mapping" => {
                if let (Some(customer), Some(supplier)) = (&doc.customer, &doc.supplier)
                    && let (Some(cust_bc), Some(supp_bc)) =
                        (&customer.bounded_context, &supplier.bounded_context)
                {
                    links.push(GraphLink {
                        source: format!("bounded-context:{supp_bc}"),
                        target: format!("bounded-context:{cust_bc}"),
                        link_type: "supplier-to-customer".to_string(),
                    });
                }
            }
            "process-manager" => {
                let pm_id = format!("process-manager:{}", doc.name);
                nodes.push(GraphNode {
                    id: pm_id.clone(),
                    name: doc.name.clone(),
                    kind: "process-manager".to_string(),
                    bounded_context: None,
                    description: doc.description.clone(),
                });

                if let Some(starts_when) = &doc.starts_when {
                    for start in starts_when {
                        let start_bc = start
                            .bounded_context
                            .clone()
                            .unwrap_or_else(|| "global".to_string());
                        if let Some(ref event_name) = start.event {
                            links.push(GraphLink {
                                source: format!("event:{start_bc}:{event_name}"),
                                target: pm_id.clone(),
                                link_type: "starts-when".to_string(),
                            });
                        }
                    }
                }

                if let Some(reactions) = &doc.reactions {
                    for reaction in reactions {
                        if let Some(ref when) = reaction.when {
                            let when_bc = when
                                .bounded_context
                                .clone()
                                .unwrap_or_else(|| "global".to_string());
                            if let Some(ref event_name) = when.event {
                                links.push(GraphLink {
                                    source: format!("event:{when_bc}:{event_name}"),
                                    target: pm_id.clone(),
                                    link_type: "reacts-to".to_string(),
                                });
                            }
                        }
                        if let Some(emits) = &reaction.emits {
                            for emit in emits {
                                let emit_bc = emit
                                    .bounded_context
                                    .clone()
                                    .unwrap_or_else(|| "global".to_string());
                                if let Some(ref command_name) = emit.command {
                                    links.push(GraphLink {
                                        source: pm_id.clone(),
                                        target: format!("command:{emit_bc}:{command_name}"),
                                        link_type: "emits".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let graph = DomainGraph {
        domain: GraphDomain {
            name: domain_name,
            description: domain_desc,
        },
        nodes,
        links,
    };

    let json_content = serde_json::to_string_pretty(&graph)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, json_content)?;

    Ok(())
}

pub fn run_visualizer_server(input_path: &std::path::Path) -> Result<()> {
    use anyhow::Context;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let json_content = std::fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read graph JSON: {input_path:?}"))?;

    let html_content = include_str!("visualizer.html");

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{port}");

    println!("====================================================");
    println!("  Chromatofoor Domain Visualizer");
    println!("  Server running at: {url}");
    println!("  Press Ctrl+C to stop.");
    println!("====================================================");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut buffer = [0; 2048];
        let bytes_read = stream.read(&mut buffer)?;
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);

        let response = if request.contains("GET /graph.json") {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_content.len(),
                json_content
            )
        } else {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html_content.len(),
                html_content
            )
        };
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
    }

    Ok(())
}
