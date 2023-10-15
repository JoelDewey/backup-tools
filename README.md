# backup-tools

An application to backup PostgreSQL databases and files for use within my home Kubernetes cluster.

* Automatically scales down a given `Deployment` prior to performing backups and will scale it back to the original 
  number of replicas after it has finished performing the backup operation.
* Can connect to a PostgreSQL server and make a backup, using `pg_dump` of a given database.
* Creates new backups using `rsync`, using hard links to save on storage usage.
* Can automatically rotate out older backups as newer ones are created.

The application itself is written using Rust and is deployable via a provided Helm chart. Please see 
[the documentation and code for the application](app/backup-tools) to learn more about how to build, run, and configure 
the application and then read through [the Helm chart and its documentation](charts/backup-tools) to learn more about 
how to deploy the application to a Kubernetes cluster.