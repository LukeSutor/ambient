"""
Visualization utilities for evaluation results.
"""
import json
import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
import numpy as np
from typing import Dict, Any, List, Optional
import logging

logger = logging.getLogger(__name__)

class EvalVisualizer:
    """Generate visualizations for evaluation results."""
    
    def __init__(self, style: str = 'whitegrid'):
        """Initialize visualizer with seaborn style."""
        sns.set_style(style)
        plt.rcParams['figure.figsize'] = (12, 8)
    
    def load_results(self, results_path: str) -> Dict[str, Any]:
        """Load evaluation results from JSON file."""
        with open(results_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def create_score_distribution(self, results_data: Dict[str, Any], 
                                save_path: Optional[str] = None) -> plt.Figure:
        """Create score distribution plots."""
        
        individual_results = results_data['individual_results']
        
        # Extract scores
        scores_data = []
        for result in individual_results:
            scores = result['scores']
            for metric, score in scores.items():
                scores_data.append({
                    'metric': metric,
                    'score': score,
                    'data_point': result['data_point_id']
                })
        
        df = pd.DataFrame(scores_data)
        
        # Create subplot for each metric
        fig, axes = plt.subplots(2, 3, figsize=(15, 10))
        fig.suptitle('Score Distributions by Metric', fontsize=16)
        
        metrics = ['relevance', 'specificity', 'completeness', 'accuracy', 'overall']
        
        for i, metric in enumerate(metrics):
            row = i // 3
            col = i % 3
            ax = axes[row, col]
            
            metric_data = df[df['metric'] == metric]['score']
            
            # Histogram
            ax.hist(metric_data, bins=20, alpha=0.7, edgecolor='black')
            ax.set_title(f'{metric.title()} Score Distribution')
            ax.set_xlabel('Score')
            ax.set_ylabel('Frequency')
            ax.set_xlim(0, 10)
            
            # Add mean line
            mean_score = metric_data.mean()
            ax.axvline(mean_score, color='red', linestyle='--', 
                      label=f'Mean: {mean_score:.2f}')
            ax.legend()
        
        # Remove empty subplot
        axes[1, 2].remove()
        
        plt.tight_layout()
        
        if save_path:
            plt.savefig(save_path, dpi=300, bbox_inches='tight')
            logger.info(f"Score distribution plot saved to: {save_path}")
        
        return fig
    
    def create_metric_comparison(self, results_data: Dict[str, Any],
                               save_path: Optional[str] = None) -> plt.Figure:
        """Create metric comparison boxplot."""
        
        individual_results = results_data['individual_results']
        
        # Prepare data
        scores_data = []
        for result in individual_results:
            scores = result['scores']
            for metric, score in scores.items():
                if metric != 'overall':  # Exclude overall for cleaner comparison
                    scores_data.append({
                        'Metric': metric.title(),
                        'Score': score
                    })
        
        df = pd.DataFrame(scores_data)
        
        # Create boxplot
        fig, ax = plt.subplots(figsize=(10, 6))
        
        sns.boxplot(data=df, x='Metric', y='Score', ax=ax)
        sns.swarmplot(data=df, x='Metric', y='Score', ax=ax, 
                     size=4, alpha=0.6, color='black')
        
        ax.set_title('Task Detection Evaluation Metrics Comparison')
        ax.set_ylabel('Score (0-10)')
        ax.set_ylim(0, 10)
        
        # Add grid
        ax.grid(True, alpha=0.3)
        
        plt.tight_layout()
        
        if save_path:
            plt.savefig(save_path, dpi=300, bbox_inches='tight')
            logger.info(f"Metric comparison plot saved to: {save_path}")
        
        return fig
    
    def create_performance_summary(self, results_data: Dict[str, Any],
                                 save_path: Optional[str] = None) -> plt.Figure:
        """Create performance summary dashboard."""
        
        aggregate_metrics = results_data['aggregate_metrics']
        individual_results = results_data['individual_results']
        
        fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 12))
        fig.suptitle('Task Detection Evaluation Summary', fontsize=16)
        
        # 1. Average scores bar chart
        metrics = ['relevance', 'specificity', 'completeness', 'accuracy', 'overall']
        avg_scores = [aggregate_metrics[f'avg_{metric}'] for metric in metrics]
        std_scores = [aggregate_metrics[f'std_{metric}'] for metric in metrics]
        
        bars = ax1.bar(metrics, avg_scores, yerr=std_scores, capsize=5, 
                      alpha=0.7, edgecolor='black')
        ax1.set_title('Average Scores by Metric')
        ax1.set_ylabel('Average Score')
        ax1.set_ylim(0, 10)
        
        # Add value labels on bars
        for bar, avg in zip(bars, avg_scores):
            ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.1,
                    f'{avg:.2f}', ha='center', va='bottom')
        
        # 2. Issues summary
        total_missed = aggregate_metrics.get('total_missed_tasks', 0)
        total_false_pos = aggregate_metrics.get('total_false_positives', 0)
        total_evaluated = aggregate_metrics.get('total_evaluated', 1)
        
        issues_data = [total_missed, total_false_pos]
        issues_labels = ['Missed Tasks', 'False Positives']
        
        ax2.pie(issues_data, labels=issues_labels, autopct='%1.1f%%', startangle=90)
        ax2.set_title(f'Issues Distribution\n(Total: {sum(issues_data)})')
        
        # 3. Score correlation heatmap
        score_correlations = []
        for result in individual_results:
            scores = result['scores']
            score_correlations.append([scores[m] for m in metrics])
        
        corr_df = pd.DataFrame(score_correlations, columns=metrics)
        correlation_matrix = corr_df.corr()
        
        sns.heatmap(correlation_matrix, annot=True, cmap='coolwarm', center=0,
                   square=True, ax=ax3)
        ax3.set_title('Score Correlations')
        
        # 4. Task count analysis
        task_counts = [result['detected_tasks_count'] for result in individual_results]
        
        ax4.hist(task_counts, bins=15, alpha=0.7, edgecolor='black')
        ax4.set_title('Detected Tasks Count Distribution')
        ax4.set_xlabel('Number of Detected Tasks')
        ax4.set_ylabel('Frequency')
        
        # Add statistics
        mean_tasks = np.mean(task_counts)
        median_tasks = np.median(task_counts)
        ax4.axvline(mean_tasks, color='red', linestyle='--', 
                   label=f'Mean: {mean_tasks:.1f}')
        ax4.axvline(median_tasks, color='green', linestyle='--',
                   label=f'Median: {median_tasks:.1f}')
        ax4.legend()
        
        plt.tight_layout()
        
        if save_path:
            plt.savefig(save_path, dpi=300, bbox_inches='tight')
            logger.info(f"Performance summary saved to: {save_path}")
        
        return fig
    
    def generate_all_plots(self, results_path: str, output_dir: str = './plots'):
        """Generate all visualization plots."""
        import os
        
        # Create output directory
        os.makedirs(output_dir, exist_ok=True)
        
        # Load results
        results_data = self.load_results(results_path)
        
        # Generate plots
        self.create_score_distribution(
            results_data, 
            os.path.join(output_dir, 'score_distributions.png')
        )
        
        self.create_metric_comparison(
            results_data,
            os.path.join(output_dir, 'metric_comparison.png') 
        )
        
        self.create_performance_summary(
            results_data,
            os.path.join(output_dir, 'performance_summary.png')
        )
        
        logger.info(f"All plots generated in: {output_dir}")

if __name__ == '__main__':
    # Example usage
    import sys
    
    if len(sys.argv) != 2:
        print("Usage: python visualize.py <results_json_path>")
        sys.exit(1)
    
    results_path = sys.argv[1]
    
    visualizer = EvalVisualizer()
    visualizer.generate_all_plots(results_path)
