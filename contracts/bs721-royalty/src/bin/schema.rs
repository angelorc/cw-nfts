use std::{env::current_dir, fs::create_dir_all};

use cosmwasm_schema::{remove_schemas, export_schema, schema_for, export_schema_with_title};

use bs721_royalty::{msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ConfigResponse}, state::Config};
use cw4::{MemberResponse, MemberListResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(MemberResponse), &out_dir);
    export_schema_with_title(
        &schema_for!(MemberListResponse),
        &out_dir,
        "ListMembersResponse",
    );
}
