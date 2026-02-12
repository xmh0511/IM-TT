use sea_orm::{Database, DatabaseConnection, DbErr};

pub type DbConn = DatabaseConnection;

pub async fn create_connection(database_url: &str) -> Result<DbConn, DbErr> {
    let db = Database::connect(database_url).await?;
    Ok(db)
}

pub async fn run_migrations(db: &DbConn) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, Statement};
    
    // Create tables if they don't exist
    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            username VARCHAR(50) NOT NULL UNIQUE,
            email VARCHAR(100) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            avatar VARCHAR(255),
            status VARCHAR(20) DEFAULT 'offline',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_username (username),
            INDEX idx_email (email)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
        "#.to_owned()
    )).await?;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE TABLE IF NOT EXISTS groups_table (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            description TEXT,
            avatar VARCHAR(255),
            owner_id BIGINT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE,
            INDEX idx_owner (owner_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
        "#.to_owned()
    )).await?;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE TABLE IF NOT EXISTS group_members (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            group_id BIGINT NOT NULL,
            user_id BIGINT NOT NULL,
            role VARCHAR(20) DEFAULT 'member',
            joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (group_id) REFERENCES groups_table(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE KEY unique_member (group_id, user_id),
            INDEX idx_group (group_id),
            INDEX idx_user (user_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
        "#.to_owned()
    )).await?;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            sender_id BIGINT NOT NULL,
            receiver_id BIGINT,
            group_id BIGINT,
            content TEXT NOT NULL,
            message_type VARCHAR(20) DEFAULT 'text',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            is_read BOOLEAN DEFAULT FALSE,
            FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (receiver_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (group_id) REFERENCES groups_table(id) ON DELETE CASCADE,
            INDEX idx_sender (sender_id),
            INDEX idx_receiver (receiver_id),
            INDEX idx_group (group_id),
            INDEX idx_created (created_at)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
        "#.to_owned()
    )).await?;

    Ok(())
}
