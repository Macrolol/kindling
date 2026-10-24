import re,sys
cur=None
for line in open(sys.argv[1]):
    m=re.match(r'seed (\d+): Y(\d+) pop (\d+).*?\| ([\d.]+)s \(gen',line)
    if m:
        st=re.search(r'stage now: (\w+)',line).group(1)
        cur=dict(s=m.group(1),pop=m.group(3),t=m.group(4),st=st);continue
    if cur is None: continue
    if 'continents settled' in line:
        cur['cont']=re.search(r'settled (\d+/\d+)',line).group(1)
        cur['cy']=' '.join(y for y in re.findall(r'(Y\d+) pop',line) if y!='Y0')
    elif re.match(r'\s+sea:',line):
        cur['lost']=re.search(r'lost (\d+) people',line).group(1)
        cur['voy']=re.search(r'voyages (\d+)',line).group(1)
        cur['land']=re.search(r'landed on another landmass (\d+)',line).group(1)
    elif re.match(r'\s+conflict:',line):
        cur['war']=re.search(r'killed = ([\d.]+%)',line).group(1)
        cur['dis']=re.search(r'disease deaths ([\d.]+%)',line).group(1)
        cur['lin']=re.search(r'largest (\d+%) of people; by century (\[[^\]]*\])',line).groups()
        cur['popc']=re.search(r'pop by century (\[[^\]]*\])',line).group(1)
    elif re.match(r'\s+farming:',line):
        cur['orig']=re.search(r'independent origins (\d+)',line).group(1)
        cur['dom']=re.search(r'domestic grain (\S+)',line).group(1)
        cur['herd']=re.search(r'herding (\S+)',line).group(1)
    elif re.match(r'\s+farming now:',line):
        fb=re.search(r'farming now: (.*?) \| landmasses',line).group(1)
        fc=re.search(r'by century farmer pop/pop food% (\[.*\])',line).group(1)
        c=cur
        print(f"s{c['s']} pop {c['pop']} {c['t']}s {c['st']} | cont {c['cont']} {c.get('cy','')} | voy {c['voy']} land {c['land']} lost {c['lost']} | war {c['war']} dis {c['dis']} | lin {c['lin'][0]} {c['lin'][1]} | orig {c['orig']} dom {c['dom']} herd {c['herd']}\n    {fb}\n    farm {fc}\n    pop {c['popc']}")
        cur=None
for line in open(sys.argv[1]):
    if 'sea (all' in line or 'conflict (mean' in line or 'Kendall' in line: print(line.rstrip()[:300])
