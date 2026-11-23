let REFL_TAC ?(args_str:string list option) ?(caller_linenum:(string * int) option) : tactic =
  fun (trace_asl,trace_g) ->
    let trace_goal_before = (trace_asl,trace_g) in
    let (trace_ga_inst,trace_goals_after,trace_justf) =
      exptrace_guard "REFL_TAC" trace_goal_before (fun () -> REFL_TAC (trace_asl,trace_g)) in
    exptrace_add_tac "REFL_TAC" {
      definition_line_number = ("hol.ml",120);
      user_line_number = (match caller_linenum with
          | None -> ("",0)
          | Some t -> t
        );
      goal_before = trace_goal_before;
      goals_after = trace_goals_after;
      num_subgoals = List.length trace_goals_after;
    } (fun () -> {
      names = to_n_elems [] 0;
      types = [];
      values = [];
      exprs = to_n_elems
         (match args_str with | Some s -> s  | None -> [])
         0;
    });
    (trace_ga_inst,trace_goals_after,trace_justf);;
let REWRITE_TAC ?(args_str:string list option) ?(caller_linenum:(string * int) option) (thms:thm list) : tactic =
  fun (trace_asl,trace_g) ->
    let trace_goal_before = (trace_asl,trace_g) in
    let (trace_ga_inst,trace_goals_after,trace_justf) =
      exptrace_guard "REWRITE_TAC" trace_goal_before (fun () -> REWRITE_TAC thms (trace_asl,trace_g)) in
    exptrace_add_tac "REWRITE_TAC" {
      definition_line_number = ("hol.ml",245);
      user_line_number = (match caller_linenum with
          | None -> ("",0)
          | Some t -> t
        );
      goal_before = trace_goal_before;
      goals_after = trace_goals_after;
      num_subgoals = List.length trace_goals_after;
    } (fun () -> {
      names = to_n_elems ["thms"] 1;
      types = ["thm list"];
      values = [
        "[" ^ (String.concat ", " (List.map (fun x -> ((let s = string_of_thm x in if String.length s > 1000 then "(omitted thm)" else s)) ^ ")") thms)) ^ "]"
      ];
      exprs = to_n_elems
         (match args_str with | Some s -> s  | None -> [])
         1;
    });
    (trace_ga_inst,trace_goals_after,trace_justf);;
let BETA_CONV ?(args_str:string list option) ?(caller_linenum:(string * int) option) (tm:term) : thm =
  let thm_output =
    exptrace_guard_conv "BETA_CONV" (fun () -> tm) (fun () -> BETA_CONV tm) in
  exptrace_add_conv "BETA_CONV" {
    definition_line_number = ("hol.ml",310);
    user_line_number = (match caller_linenum with
        | None -> ("",0)
        | Some t -> t
      );
    input = tm;
    output = thm_output;
  } (fun () -> {
    names = to_n_elems ["tm"] 1;
    types = ["term"];
    values = [
      "`" ^ (string_of_term tm) ^ "`"
    ];
    exprs = to_n_elems
       (match args_str with | Some s -> s  | None -> [])
       1;
  });
  thm_output;;
