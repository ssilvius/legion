(function () {
  "use strict";

  var tabs = document.querySelectorAll(".tab");
  var content = document.getElementById("content");

  // -- State --

  var state = {
    agents: [],
    feed: [],
    tasks: [],
    stats: [],
    signals: [],
    chatMessages: [],
    chatAgent: "",
    feedFilter: "all",
    feedRepo: "all",
    taskFormOpen: false,
  };

  // -- Helpers --

  function relativeTime(isoStr) {
    if (!isoStr) return "";
    var then = new Date(isoStr).getTime();
    var now = Date.now();
    var diff = Math.floor((now - then) / 1000);
    if (diff < 0) return "just now";
    if (diff < 60) return diff + "s ago";
    if (diff < 3600) return Math.floor(diff / 60) + "m ago";
    if (diff < 86400) return Math.floor(diff / 3600) + "h ago";
    return Math.floor(diff / 86400) + "d ago";
  }

  function truncate(text, max) {
    if (text.length <= max) return text;
    return text.slice(0, max) + "...";
  }

  function el(tag, className, textContent) {
    var e = document.createElement(tag);
    if (className) e.className = className;
    if (textContent !== undefined) e.textContent = textContent;
    return e;
  }

  function isPathLike(name) {
    return name.indexOf("/") !== -1 || name.indexOf("agent-") === 0;
  }

  function unreadClass(count) {
    if (count === 0) return "unread-green";
    if (count <= 5) return "unread-yellow";
    return "unread-red";
  }

  function priorityBadgeClass(priority) {
    if (priority === "high") return "badge badge-red";
    if (priority === "med") return "badge badge-yellow";
    return "badge badge-gray";
  }

  function verbColor(verb) {
    var map = {
      question: "blue",
      review: "green",
      blocker: "red",
      request: "yellow",
      announce: "purple",
    };
    return map[verb] || "gray";
  }

  function postToApi(url, body) {
    return fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  // -- Tab routing --

  function currentTab() {
    var hash = window.location.hash.replace("#", "");
    return hash || "feed";
  }

  function updateActive() {
    var current = currentTab();
    tabs.forEach(function (tab) {
      var target = tab.getAttribute("href").replace("#", "");
      if (target === current) {
        tab.classList.add("active");
      } else {
        tab.classList.remove("active");
      }
    });
  }

  function renderPlaceholder(name) {
    return el("p", "placeholder", name + " -- coming soon");
  }

  // -- Agents tab --

  function renderAgents() {
    var container = el("div");
    var grid = el("div", "agent-grid");

    var agents = state.agents
      .filter(function (a) { return !isPathLike(a.repo); })
      .sort(function (a, b) {
        if (a.last_activity > b.last_activity) return -1;
        if (a.last_activity < b.last_activity) return 1;
        return 0;
      });

    if (agents.length === 0) {
      return el("p", "placeholder", "No agents found");
    }

    agents.forEach(function (agent) {
      var card = el("div", "agent-card");

      var header = el("div", "agent-card-header");
      var name = el("span", "agent-card-name", agent.repo);
      var badge = el("span", "unread-badge " + unreadClass(agent.unread), String(agent.unread));
      header.appendChild(name);
      header.appendChild(badge);
      card.appendChild(header);

      var stats = el("div", "agent-card-stats");

      var refStat = el("span", "agent-card-stat");
      refStat.appendChild(el("span", null, "reflections:"));
      refStat.appendChild(el("span", "agent-card-stat-value", String(agent.reflection_count)));
      stats.appendChild(refStat);

      var boostStat = el("span", "agent-card-stat");
      boostStat.appendChild(el("span", null, "boosts:"));
      boostStat.appendChild(el("span", "agent-card-stat-value", String(agent.boost_sum)));
      stats.appendChild(boostStat);

      var teamStat = el("span", "agent-card-stat");
      teamStat.appendChild(el("span", null, "posts:"));
      teamStat.appendChild(el("span", "agent-card-stat-value", String(agent.team_post_count)));
      stats.appendChild(teamStat);

      card.appendChild(stats);

      var time = el("div", "agent-card-time", relativeTime(agent.last_activity));
      card.appendChild(time);

      grid.appendChild(card);
    });

    container.appendChild(grid);
    return container;
  }

  // -- Feed tab --

  function renderFeed() {
    var container = el("div");

    // Command input
    var bar = el("div", "broadcast-bar");
    var barRow = el("div", "broadcast-row");

    var textarea = document.createElement("textarea");
    textarea.className = "broadcast-input";
    textarea.placeholder = "@agent message... or just post to the bullpen";
    textarea.rows = 2;
    barRow.appendChild(textarea);

    // Hint line
    var hint = el("div", "broadcast-hint");
    textarea.addEventListener("input", function () {
      var text = textarea.value.trim();
      var match = text.match(/^@(\w+)\s/);
      if (match) {
        hint.textContent = "signal to " + match[1] + " as meatbag";
        hint.className = "broadcast-hint active";
      } else if (text) {
        hint.textContent = "post to bullpen as meatbag";
        hint.className = "broadcast-hint active";
      } else {
        hint.textContent = "";
        hint.className = "broadcast-hint";
      }
    });
    barRow.appendChild(hint);

    var barActions = el("div", "broadcast-actions");

    var postBtn = el("button", "broadcast-btn", "Send");
    postBtn.addEventListener("click", function () {
      var text = textarea.value.trim();
      if (!text) return;
      postBtn.disabled = true;
      postToApi("/api/post", { repo: "meatbag", text: text })
        .then(function (r) {
          if (r.ok) {
            textarea.value = "";
            hint.textContent = "";
            hint.className = "broadcast-hint";
          } else {
            console.error("[legion] post failed:", r.status);
          }
        })
        .catch(function (err) {
          console.error("[legion] post error:", err);
        })
        .finally(function () {
          postBtn.disabled = false;
        });
    });

    // Ctrl+Enter to send
    textarea.addEventListener("keydown", function (e) {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        postBtn.click();
      }
    });

    barActions.appendChild(postBtn);
    barRow.appendChild(barActions);
    bar.appendChild(barRow);
    container.appendChild(bar);

    // Controls row
    var controls = el("div", "feed-controls");

    var filters = ["all", "signals", "musings"];
    filters.forEach(function (f) {
      var btn = el("button", "feed-btn" + (state.feedFilter === f ? " active" : ""), f);
      btn.addEventListener("click", function () {
        state.feedFilter = f;
        render();
      });
      controls.appendChild(btn);
    });

    // Repo dropdown
    var repos = [];
    state.feed.forEach(function (item) {
      if (repos.indexOf(item.repo) === -1) repos.push(item.repo);
    });
    repos.sort();

    var select = document.createElement("select");
    var allOpt = document.createElement("option");
    allOpt.value = "all";
    allOpt.textContent = "all repos";
    select.appendChild(allOpt);
    repos.forEach(function (r) {
      var opt = document.createElement("option");
      opt.value = r;
      opt.textContent = r;
      if (state.feedRepo === r) opt.selected = true;
      select.appendChild(opt);
    });
    select.addEventListener("change", function () {
      state.feedRepo = select.value;
      render();
    });
    controls.appendChild(select);

    container.appendChild(controls);

    // Feed list
    var list = el("div", "feed-list");
    list.id = "feed-list-container";
    list.appendChild(buildFeedItems());
    container.appendChild(list);
    return container;
  }

  // -- Tasks tab (kanban) --

  function getAgentNames() {
    var names = [];
    state.agents.forEach(function (a) {
      if (!isPathLike(a.repo)) names.push(a.repo);
    });
    names.sort();
    return names;
  }

  function renderTaskForm() {
    var wrapper = el("div", "task-form-wrapper");

    var toggleBtn = el("button", "task-form-toggle", state.taskFormOpen ? "Cancel" : "+ New Task");
    toggleBtn.addEventListener("click", function () {
      state.taskFormOpen = !state.taskFormOpen;
      render();
    });
    wrapper.appendChild(toggleBtn);

    if (!state.taskFormOpen) return wrapper;

    var form = el("div", "task-form");

    // To field
    var toLabel = el("label", "task-form-label", "Assign to");
    var toSelect = document.createElement("select");
    toSelect.className = "task-form-select";
    var agents = getAgentNames();
    agents.forEach(function (name) {
      var opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      toSelect.appendChild(opt);
    });
    form.appendChild(toLabel);
    form.appendChild(toSelect);

    // Text field
    var textLabel = el("label", "task-form-label", "Task description");
    var textArea = document.createElement("textarea");
    textArea.className = "task-form-textarea";
    textArea.rows = 3;
    textArea.placeholder = "What needs to be done?";
    form.appendChild(textLabel);
    form.appendChild(textArea);

    // Priority field
    var prioLabel = el("label", "task-form-label", "Priority");
    var prioSelect = document.createElement("select");
    prioSelect.className = "task-form-select";
    ["low", "med", "high"].forEach(function (p) {
      var opt = document.createElement("option");
      opt.value = p;
      opt.textContent = p;
      if (p === "med") opt.selected = true;
      prioSelect.appendChild(opt);
    });
    form.appendChild(prioLabel);
    form.appendChild(prioSelect);

    // Context field
    var ctxLabel = el("label", "task-form-label", "Context (optional)");
    var ctxArea = document.createElement("textarea");
    ctxArea.className = "task-form-textarea";
    ctxArea.rows = 2;
    ctxArea.placeholder = "Additional context...";
    form.appendChild(ctxLabel);
    form.appendChild(ctxArea);

    // Error display
    var errorEl = el("div", "task-form-error");
    form.appendChild(errorEl);

    // Submit
    var submitBtn = el("button", "broadcast-btn", "Create Task");
    submitBtn.addEventListener("click", function () {
      var text = textArea.value.trim();
      if (!text) {
        errorEl.textContent = "Task description is required";
        return;
      }
      submitBtn.disabled = true;
      errorEl.textContent = "";
      var body = {
        from: "meatbag",
        to: toSelect.value,
        text: text,
        priority: prioSelect.value,
        context: ctxArea.value.trim() || null,
      };
      postToApi("/api/tasks/create", body)
        .then(function (r) {
          if (r.ok) {
            state.taskFormOpen = false;
            render();
          } else {
            return r.json().then(function (data) {
              errorEl.textContent = data.error || "Failed to create task";
            });
          }
        })
        .catch(function (err) {
          errorEl.textContent = "Network error: " + err;
        })
        .finally(function () {
          submitBtn.disabled = false;
        });
    });
    form.appendChild(submitBtn);

    wrapper.appendChild(form);
    return wrapper;
  }

  function renderTasks() {
    var container = el("div");

    container.appendChild(renderTaskForm());

    var kanban = el("div", "kanban");
    kanban.id = "kanban-container";
    kanban.appendChild(buildKanbanColumns());
    container.appendChild(kanban);
    return container;
  }

  // -- Stats tab --

  function renderStats() {
    var container = el("div");
    var table = el("table", "stats-table");

    var thead = document.createElement("thead");
    var headerRow = document.createElement("tr");
    ["Repo", "Reflections", "Boosts", "Team Posts", "Last Active"].forEach(function (h) {
      headerRow.appendChild(el("th", null, h));
    });
    thead.appendChild(headerRow);
    table.appendChild(thead);

    var tbody = document.createElement("tbody");

    var rows = state.stats
      .filter(function (s) { return !isPathLike(s.repo); })
      .sort(function (a, b) {
        if (a.last_activity > b.last_activity) return -1;
        if (a.last_activity < b.last_activity) return 1;
        return 0;
      });

    rows.forEach(function (s) {
      var tr = document.createElement("tr");
      tr.appendChild(el("td", null, s.repo));
      tr.appendChild(el("td", null, String(s.reflection_count)));
      tr.appendChild(el("td", null, String(s.boost_sum)));
      tr.appendChild(el("td", null, String(s.team_post_count)));
      tr.appendChild(el("td", null, relativeTime(s.last_activity)));
      tbody.appendChild(tr);
    });

    table.appendChild(tbody);
    container.appendChild(table);

    if (rows.length === 0) {
      container.appendChild(el("p", "placeholder", "No stats available"));
    }

    return container;
  }

  // -- Signals tab --

  function renderSignals() {
    var container = el("div");

    if (state.signals.length === 0) {
      return el("p", "placeholder", "No signals");
    }

    // Group by recipient
    var groups = {};
    state.signals.forEach(function (sig) {
      var key = sig.to;
      if (!groups[key]) groups[key] = [];
      groups[key].push(sig);
    });

    var recipients = Object.keys(groups).sort();

    recipients.forEach(function (recipient) {
      var section = el("div", "signal-group");
      var header = el("div", "signal-group-header", "@" + recipient);
      section.appendChild(header);

      var list = el("div", "signal-list");

      groups[recipient].forEach(function (sig) {
        var color = verbColor(sig.verb);
        var card = el("div", "signal-card signal-border-" + color);

        var cardHeader = el("div", "signal-card-header");
        cardHeader.appendChild(el("span", "badge badge-green", sig.from_repo));
        cardHeader.appendChild(el("span", "signal-arrow", "->"));
        cardHeader.appendChild(el("span", "badge badge-" + color, sig.verb));
        if (sig.status) {
          cardHeader.appendChild(el("span", "badge badge-gray", sig.status));
        }
        cardHeader.appendChild(el("span", "signal-time", relativeTime(sig.created_at)));
        card.appendChild(cardHeader);

        var text = el("div", "signal-card-text", sig.text);
        card.appendChild(text);

        list.appendChild(card);
      });

      section.appendChild(list);
      container.appendChild(section);
    });

    return container;
  }

  // -- Chat tab --

  function fetchChat(agent) {
    if (!agent) return;
    fetch("/api/chat?agent=" + encodeURIComponent(agent))
      .then(function (r) { return r.json(); })
      .then(function (data) {
        state.chatMessages = data;
        if (currentTab() === "chat") {
          if (!updateChatMessages()) render();
        }
      })
      .catch(function (err) {
        console.error("[legion] chat fetch error:", err);
      });
  }

  function renderChat() {
    var container = el("div", "chat-container");

    // Agent selector
    var header = el("div", "chat-header");
    var label = el("span", null, "Chat with: ");
    header.appendChild(label);

    var select = document.createElement("select");
    select.className = "task-form-select";
    var emptyOpt = document.createElement("option");
    emptyOpt.value = "";
    emptyOpt.textContent = "-- select agent --";
    select.appendChild(emptyOpt);

    var agents = getAgentNames().filter(function (n) { return n !== "meatbag"; });
    agents.forEach(function (name) {
      var opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      if (state.chatAgent === name) opt.selected = true;
      select.appendChild(opt);
    });
    select.addEventListener("change", function () {
      state.chatAgent = select.value;
      state.chatMessages = [];
      fetchChat(state.chatAgent);
    });
    header.appendChild(select);
    container.appendChild(header);

    if (!state.chatAgent) {
      container.appendChild(el("p", "placeholder", "Select an agent to start a conversation"));
      return container;
    }

    // Message list
    var messageList = el("div", "chat-messages");
    messageList.id = "chat-messages-container";
    container.appendChild(messageList);

    // Populate via shared builder (also handles scroll)
    updateChatMessages();

    // Input area
    var inputBar = el("div", "chat-input");
    var textarea = document.createElement("textarea");
    textarea.className = "broadcast-input";
    textarea.placeholder = "Message @" + state.chatAgent + "...";
    textarea.rows = 2;
    inputBar.appendChild(textarea);

    var sendBtn = el("button", "broadcast-btn", "Send");
    sendBtn.addEventListener("click", function () {
      var raw = textarea.value.trim();
      if (!raw) return;
      sendBtn.disabled = true;
      // Auto-prefix @agent if not already present
      var text = raw;
      var prefix = "@" + state.chatAgent;
      if (text.toLowerCase().indexOf(prefix.toLowerCase()) === -1) {
        text = prefix + " " + text;
      }
      postToApi("/api/post", { repo: "meatbag", text: text })
        .then(function (r) {
          if (r.ok) {
            textarea.value = "";
            fetchChat(state.chatAgent);
          } else {
            console.error("[legion] chat send failed:", r.status);
          }
        })
        .catch(function (err) {
          console.error("[legion] chat send error:", err);
        })
        .finally(function () {
          sendBtn.disabled = false;
        });
    });

    textarea.addEventListener("keydown", function (e) {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        sendBtn.click();
      }
    });

    inputBar.appendChild(sendBtn);
    container.appendChild(inputBar);

    return container;
  }

  // -- Targeted update helpers --

  function buildFeedItems() {
    var frag = document.createDocumentFragment();

    var items = state.feed.filter(function (item) {
      if (state.feedRepo !== "all" && item.repo !== state.feedRepo) return false;
      if (state.feedFilter === "signals" && !item.is_signal) return false;
      if (state.feedFilter === "musings" && item.is_signal) return false;
      return true;
    });

    if (items.length === 0) {
      frag.appendChild(el("p", "placeholder", "No posts"));
    }

    items.forEach(function (item) {
      var card = el("div", "feed-item" + (item.is_signal ? " signal" : ""));

      var header = el("div", "feed-item-header");
      header.appendChild(el("span", "badge badge-green", item.repo));
      header.appendChild(el("span", null, relativeTime(item.created_at)));
      if (item.is_signal) {
        header.appendChild(el("span", "badge badge-blue", "signal"));
      }

      var boostBtn = el("button", "boost-btn", "+boost");
      (function (postId, btn) {
        btn.addEventListener("click", function () {
          btn.disabled = true;
          postToApi("/api/boost/" + postId)
            .then(function (r) {
              if (r.ok) {
                btn.textContent = "boosted";
                btn.classList.add("boosted");
              } else {
                btn.disabled = false;
              }
            })
            .catch(function () {
              btn.disabled = false;
            });
        });
      })(item.id, boostBtn);
      header.appendChild(boostBtn);

      card.appendChild(header);

      var textEl = el("div", "feed-item-text");
      var full = item.text;
      var short = truncate(full, 200);
      var expanded = false;
      textEl.textContent = short;

      if (full.length > 200) {
        var toggle = el("button", "feed-item-toggle", "show more");
        toggle.addEventListener("click", function () {
          expanded = !expanded;
          textEl.textContent = expanded ? full : short;
          toggle.textContent = expanded ? "show less" : "show more";
          card.appendChild(toggle);
        });
        card.appendChild(textEl);
        card.appendChild(toggle);
      } else {
        card.appendChild(textEl);
      }

      frag.appendChild(card);
    });

    return frag;
  }

  function updateFeedList() {
    var container = document.getElementById("feed-list-container");
    if (!container) return false;
    container.replaceChildren();
    container.appendChild(buildFeedItems());
    return true;
  }

  function buildKanbanColumns() {
    var frag = document.createDocumentFragment();
    var columns = ["pending", "accepted", "done", "blocked"];
    var labels = { pending: "Pending", accepted: "Accepted", done: "Done", blocked: "Blocked" };

    columns.forEach(function (status) {
      var col = el("div", "kanban-col");
      var tasks = state.tasks.filter(function (t) { return t.status === status; });

      var header = el("div", "kanban-col-header");
      header.textContent = labels[status] + " ";
      var count = el("span", "kanban-col-count", "(" + tasks.length + ")");
      header.appendChild(count);
      col.appendChild(header);

      var cards = el("div", "kanban-cards");

      tasks.forEach(function (task) {
        var card = el("div", "kanban-card");

        var route = el("div", "kanban-card-route", task.from_repo + " -> " + task.to_repo);
        card.appendChild(route);

        var text = el("div", "kanban-card-text", truncate(task.text, 120));
        card.appendChild(text);

        var footer = el("div", "kanban-card-footer");
        footer.appendChild(el("span", priorityBadgeClass(task.priority), task.priority));
        footer.appendChild(el("span", null, relativeTime(task.created_at)));
        card.appendChild(footer);

        cards.appendChild(card);
      });

      if (tasks.length === 0) {
        cards.appendChild(el("p", "placeholder", "none"));
      }

      col.appendChild(cards);
      frag.appendChild(col);
    });

    return frag;
  }

  function updateKanban() {
    var container = document.getElementById("kanban-container");
    if (!container) return false;
    container.replaceChildren();
    container.appendChild(buildKanbanColumns());
    return true;
  }

  function updateChatMessages() {
    var container = document.getElementById("chat-messages-container");
    if (!container) return false;
    container.replaceChildren();

    if (state.chatMessages.length === 0) {
      container.appendChild(el("p", "placeholder", "No messages with @" + state.chatAgent));
    }

    state.chatMessages.forEach(function (msg) {
      var isMeatbag = msg.repo.toLowerCase() === "meatbag";
      var msgEl = el("div", "chat-msg" + (isMeatbag ? " meatbag" : " agent"));

      var senderBadge = el("span", "badge " + (isMeatbag ? "badge-green" : "badge-blue"), msg.repo);
      var time = el("span", "chat-msg-time", relativeTime(msg.created_at));

      var msgHeader = el("div", "chat-msg-header");
      msgHeader.appendChild(senderBadge);
      msgHeader.appendChild(time);
      msgEl.appendChild(msgHeader);

      var textEl = el("div", "chat-msg-text", msg.text);
      msgEl.appendChild(textEl);

      container.appendChild(msgEl);
    });

    setTimeout(function () {
      container.scrollTop = container.scrollHeight;
    }, 0);

    return true;
  }

  // -- Renderers --

  var renderers = {
    feed: renderFeed,
    signals: renderSignals,
    tasks: renderTasks,
    stats: renderStats,
    chat: renderChat,
  };

  function render() {
    var tab = currentTab();
    var fn = renderers[tab];
    content.replaceChildren();
    if (fn) {
      content.appendChild(fn());
    } else {
      content.appendChild(renderPlaceholder("Unknown"));
    }
    updateActive();
  }

  window.addEventListener("hashchange", render);

  // -- Data fetching --

  function fetchJSON(url, key) {
    fetch(url)
      .then(function (r) { return r.json(); })
      .then(function (data) {
        state[key] = data;
        render();
      })
      .catch(function (err) {
        console.error("[legion] fetch " + url + " failed:", err);
      });
  }

  // Initial load
  fetchJSON("/api/agents", "agents");
  fetchJSON("/api/feed", "feed");
  fetchJSON("/api/tasks", "tasks");
  fetchJSON("/api/stats", "stats");
  fetchJSON("/api/signals", "signals");

  // Initial render (shows loading until data arrives)
  render();

  // -- SSE connection with reconnect --

  var retryDelay = 1000;
  var maxRetryDelay = 30000;

  function connectSSE() {
    var source = new EventSource("/sse");

    source.onopen = function () {
      retryDelay = 1000;
      console.log("[legion] SSE connected");
    };

    source.addEventListener("agents", function (event) {
      try {
        state.agents = JSON.parse(event.data);
      } catch (e) {
        console.error("[legion] agents parse error:", e);
      }
    });

    source.addEventListener("feed", function (event) {
      try {
        state.feed = JSON.parse(event.data);
        if (currentTab() === "feed") {
          if (!updateFeedList()) render();
        }
        if (currentTab() === "chat" && state.chatAgent) fetchChat(state.chatAgent);
        // Refresh signals when feed changes (signals come from feed data)
        fetch("/api/signals")
          .then(function (r) { return r.json(); })
          .then(function (data) {
            state.signals = data;
            if (currentTab() === "signals") render();
          })
          .catch(function (err) {
            console.error("[legion] signals fetch error:", err);
          });
      } catch (e) {
        console.error("[legion] feed parse error:", e);
      }
    });

    source.addEventListener("tasks", function (event) {
      try {
        state.tasks = JSON.parse(event.data);
        if (currentTab() === "tasks") {
          if (!updateKanban()) render();
        }
      } catch (e) {
        console.error("[legion] tasks parse error:", e);
      }
    });

    source.addEventListener("ping", function () {
      // keepalive, nothing to do
    });

    source.onerror = function () {
      source.close();
      console.log("[legion] SSE disconnected, retrying in " + retryDelay + "ms");
      setTimeout(connectSSE, retryDelay);
      retryDelay = Math.min(retryDelay * 2, maxRetryDelay);
    };
  }

  connectSSE();
})();
