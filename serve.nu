{|req|
  if $req.path == "/" {
    minijinja-cli "./index.html"
  } else if $req.path == "/canvas/" {
    let content = (open "canvas/content.md" | m2h)
    let value_propositions = (try { open "canvas/sections/value-propositions.md" | m2h } catch { "" })

    {
      content: $content
      sections: {
        value_propositions: $value_propositions
      }
    } | to json -r | minijinja-cli "./canvas/index.html" --format json -
  } else {
    .static "." $req.path
  }
}
