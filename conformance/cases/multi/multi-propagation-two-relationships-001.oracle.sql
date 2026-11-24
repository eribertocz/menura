SELECT COUNT(tickets.ticket_id) AS ticket_count
FROM shop.tickets AS tickets
WHERE EXISTS (
    SELECT 1 FROM shop.desks AS desks
    WHERE desks.desk_id = tickets.desk_id AND desks.region IN ('NORTH')
  )
  AND NOT EXISTS (
    SELECT 1 FROM shop.desks AS desks
    WHERE desks.desk_id = tickets.desk_id AND NOT (desks.region IN ('NORTH'))
  )
  AND EXISTS (
    SELECT 1 FROM shop.queues AS queues
    WHERE queues.queue_id = tickets.queue_id AND queues.region IN ('NORTH')
  )
  AND NOT EXISTS (
    SELECT 1 FROM shop.queues AS queues
    WHERE queues.queue_id = tickets.queue_id AND NOT (queues.region IN ('NORTH'))
  )
