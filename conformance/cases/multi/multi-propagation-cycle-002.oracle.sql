SELECT COUNT(people.person_id) AS person_count
FROM shop.main.people AS people
JOIN shop.main.teams AS teams ON people.team_id = teams.team_id
WHERE teams.region IN ('SOUTH')
