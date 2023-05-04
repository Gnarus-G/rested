mod tests {
    use insta::assert_debug_snapshot;
    use rested::lexer;

    macro_rules! assert_lexer_snapshot {
        ($input:literal) => {
            let mut lexer = lexer::Lexer::new($input);

            let mut tokens = vec![];
            let mut token = lexer.next();
            while token.kind != lexer::TokenKind::End {
                tokens.push(token);
                token = lexer.next();
            }
            assert_debug_snapshot!(tokens);
        };
    }
    #[test]
    fn lex_put_patch_delete() {
        assert_lexer_snapshot!(
            r#"
put /api {}
patch /api {}
delete /api {}
"#
        );
    }
}
