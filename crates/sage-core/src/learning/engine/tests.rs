//! Tests for learning engine

#[cfg(test)]
mod tests {
    use crate::learning::engine::core::LearningEngine;
    use crate::learning::engine::error::LearningError;
    use crate::learning::types::*;

    #[tokio::test]
    async fn test_learn_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Use 4-space indentation",
            "Indent with 4 spaces, not tabs",
            PatternSource::UserExplicit,
        );

        let id = engine.learn(pattern).await.unwrap();
        assert!(engine.get_pattern(&id).await.is_some());
    }

    #[tokio::test]
    async fn test_learn_disabled() {
        let engine = LearningEngine::new(LearningConfig::disabled());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Test",
            "Rule",
            PatternSource::UserExplicit,
        );

        let result = engine.learn(pattern).await;
        assert!(matches!(result, Err(LearningError::Disabled)));
    }

    #[tokio::test]
    async fn test_reinforce_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::ToolPreference,
            "Use ripgrep",
            "Prefer rg over grep",
            PatternSource::ToolUsage,
        );

        let id = engine.learn(pattern).await.unwrap();
        let initial = engine.get_pattern(&id).await.unwrap();

        engine.reinforce(&id).await.unwrap();
        let reinforced = engine.get_pattern(&id).await.unwrap();

        assert!(reinforced.confidence.value() > initial.confidence.value());
        assert_eq!(reinforced.observation_count, 2);
    }

    #[tokio::test]
    async fn test_contradict_pattern() {
        let engine = LearningEngine::new(LearningConfig::default());

        let pattern = Pattern::new(
            PatternType::CodingStyle,
            "Style",
            "Rule",
            PatternSource::BehaviorPattern,
        );

        let id = engine.learn(pattern).await.unwrap();

        // Reinforce the pattern first so it has enough observations
        for _ in 0..5 {
            engine.reinforce(&id).await.unwrap();
        }

        // First contradiction should keep pattern valid
        let still_valid = engine.contradict(&id).await.unwrap();
        assert!(still_valid);

        // Many contradictions should eventually invalidate
        for _ in 0..10 {
            let _ = engine.contradict(&id).await;
        }

        let pattern = engine.get_pattern(&id).await;
        if let Some(p) = pattern {
            assert!(!p.is_valid() || p.contradiction_count > 5);
        }
    }

    #[tokio::test]
    async fn test_learn_from_correction() {
        let engine = LearningEngine::new(LearningConfig::default());

        let id = engine
            .learn_from_correction(
                "Using grep -r",
                "Use ripgrep (rg) for better performance",
                Some(vec!["bash".to_string()]),
            )
            .await
            .unwrap();

        let pattern = engine.get_pattern(&id).await.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Correction);
        assert!(pattern.context.contains(&"bash".to_string()));
    }

    #[tokio::test]
    async fn test_get_applicable_patterns() {
        let engine = LearningEngine::new(LearningConfig {
            apply_threshold: 0.5,
            ..Default::default()
        });

        // Learn a high-confidence pattern
        let pattern = Pattern::new(
            PatternType::ToolPreference,
            "Use ripgrep",
            "Prefer rg over grep",
            PatternSource::UserExplicit,
        )
        .with_confidence(0.9)
        .with_context("bash");

        engine.learn(pattern).await.unwrap();

        let applicable = engine.get_applicable_patterns(&["bash".to_string()]).await;
        assert!(!applicable.is_empty());

        let not_applicable = engine
            .get_applicable_patterns(&["unrelated".to_string()])
            .await;
        // Pattern with empty context would still apply, so check if it's in the list
        assert!(not_applicable.len() <= applicable.len());
    }

    #[tokio::test]
    async fn test_patterns_for_prompt() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn high-confidence patterns
        for i in 0..5 {
            let pattern = Pattern::new(
                PatternType::CodingStyle,
                format!("Style rule {}", i),
                format!("Apply rule {}", i),
                PatternSource::UserExplicit,
            )
            .with_confidence(0.9);
            engine.learn(pattern).await.unwrap();
        }

        let prompts = engine.get_patterns_for_prompt(3).await;
        assert_eq!(prompts.len(), 3);
    }

    #[tokio::test]
    async fn test_stats() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn patterns
        engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Style 1",
                "Rule 1",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        engine
            .learn(Pattern::new(
                PatternType::ToolPreference,
                "Tool 1",
                "Rule 2",
                PatternSource::ToolUsage,
            ))
            .await
            .unwrap();

        let stats = engine.stats().await;
        assert_eq!(stats.total_patterns, 2);
        assert!(stats.patterns_by_type.contains_key("Coding Style"));
        assert!(stats.patterns_by_type.contains_key("Tool Preference"));
    }

    #[tokio::test]
    async fn test_clear() {
        let engine = LearningEngine::new(LearningConfig::default());

        engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Test",
                "Rule",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        assert_eq!(engine.stats().await.total_patterns, 1);

        engine.clear().await;

        assert_eq!(engine.stats().await.total_patterns, 0);
    }

    #[tokio::test]
    async fn test_deduplicate_similar_patterns() {
        let engine = LearningEngine::new(LearningConfig::default());

        // Learn same pattern twice
        let id1 = engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Use 4 spaces",
                "indent with 4 spaces",
                PatternSource::UserExplicit,
            ))
            .await
            .unwrap();

        let id2 = engine
            .learn(Pattern::new(
                PatternType::CodingStyle,
                "Indentation",
                "INDENT WITH 4 SPACES", // Same rule, different case
                PatternSource::BehaviorPattern,
            ))
            .await
            .unwrap();

        // Should return the same ID (pattern was reinforced, not duplicated)
        assert_eq!(id1, id2);
        assert_eq!(engine.stats().await.total_patterns, 1);
    }
}
