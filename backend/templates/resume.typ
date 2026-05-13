// Resume template — populated with JSON via `sys.inputs.profile`.
#let profile = json(bytes(sys.inputs.profile))

#set page(
  paper: "us-letter",
  margin: (x: 0.75in, y: 0.75in),
)
#set text(font: "Linux Libertine", size: 10pt)
#set par(justify: false, leading: 0.55em)

#let accent = rgb("#2563eb")

// Header
#align(center)[
  #text(size: 22pt, weight: "bold")[Joshua Holmes]
  #v(-6pt)
  #text(size: 12pt, fill: accent)[Software Engineer]
  #v(-6pt)
  #text(size: 9pt)[contact\@holmes-software.com]
]

#v(8pt)
#line(length: 100%, stroke: 0.5pt + accent)
#v(4pt)

// About
#text(size: 13pt, weight: "bold", fill: accent)[About]
#v(2pt)
#par[#profile.about.description]

#v(8pt)

// Experience
#text(size: 13pt, weight: "bold", fill: accent)[Experience]
#v(4pt)

#for job in profile.experience [
  #grid(
    columns: (1fr, auto),
    [
      #text(weight: "bold", size: 11pt)[#job.title] \
      #text(style: "italic")[#job.company]
    ],
    [
      #align(right)[
        #text(size: 9pt, fill: gray)[
          #job.start -- #if job.end == none [Present] else [#job.end]
        ]
      ]
    ],
  )
  #v(-4pt)
  #for ach in job.achievements [
    - #ach
  ]
  #v(4pt)
]

#v(4pt)

// Projects
#text(size: 13pt, weight: "bold", fill: accent)[Projects]
#v(4pt)

#for proj in profile.projects [
  #text(weight: "bold")[#proj.title] \
  #text(size: 9pt)[#proj.description] \
  #text(size: 8pt, fill: accent)[#proj.link]
  #v(4pt)
]
