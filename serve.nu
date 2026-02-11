{|req|
  if $req.path == "/" {
    {} | .mj "./index.html"
  } else if $req.path == "/canvas/" {
    let content = (open "canvas/content.md" | .md | get __html)
    let value_propositions = (try { open "canvas/sections/value-propositions.md" | .md | get __html } catch { "" })

    {
      content: $content
      sections: {
        value_propositions: $value_propositions
      }
    } | .mj "./canvas/index.html"
  } else {
    .static (pwd) $req.path
  }
}
